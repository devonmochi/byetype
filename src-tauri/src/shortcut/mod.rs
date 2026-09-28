//! Global shortcut runtime: normalization/conflict validation, registration,
//! a centralized dispatch layer with generation guard, capture sessions and the
//! dedicated field-level update path.
//!
//! The four shortcut fields are the only concern of this module. Recording,
//! transcription, screenshot and every other business flow is left untouched;
//! the voice/screenshot handlers below are moved verbatim from the previous
//! single-file implementation.

pub mod model;
mod native;

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

use crate::audio::recorder::AudioRecorder;
use crate::config::shortcut_config::ShortcutSchemaMode;
use crate::config::ConfigManager;

pub use model::ShortcutField;
pub use native::BackendKind;

use model::{
    build_conflict_graph, is_alt_right, validate_candidates, NormalizedShortcut, ALT_RIGHT,
};

/// PTT 模式下，按住时间小于此阈值视为误触，丢弃录音。
const PTT_MIN_DURATION_MS: u64 = 300;

/// 等待麦克风送出首帧音频的轮询间隔。
const READY_POLL_INTERVAL_MS: u64 = 20;
/// 等待首帧音频的最大轮询次数（20ms × 150 = 3 秒兜底）。
const READY_POLL_TICKS: u32 = 150;

/// A capture session that is never explicitly ended expires after this long so
/// a crashed/refreshed frontend cannot suppress shortcuts forever.
const CAPTURE_MAX_AGE: Duration = Duration::from_secs(120);

// ==================== Public request/response types ====================

/// Sparse shortcut patch. `None` = do not touch; `Some("")` = clear.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShortcutPatch {
    #[serde(default)]
    pub shortcut: Option<String>,
    #[serde(default)]
    pub shortcut2: Option<String>,
    #[serde(default)]
    pub extract_shortcut: Option<String>,
    #[serde(default)]
    pub extract_shortcut2: Option<String>,
}

impl ShortcutPatch {
    pub fn is_empty(&self) -> bool {
        self.shortcut.is_none()
            && self.shortcut2.is_none()
            && self.extract_shortcut.is_none()
            && self.extract_shortcut2.is_none()
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FieldResult {
    pub field: String,
    pub value: String,
    pub registered: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShortcutUpdateResult {
    pub shortcut: String,
    pub shortcut2: String,
    pub extract_shortcut: String,
    pub extract_shortcut2: String,
    pub alt_right_backend: Option<String>,
    pub per_field: Vec<FieldResult>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShortcutStatus {
    pub schema_mode: String,
    /// True when the shortcut section may be edited without risking the config.
    pub editable: bool,
    pub alt_right_backend: Option<String>,
    /// Native right-Alt events observed at the source (hook/raw input).
    pub native_events_seen: u64,
    /// Native events that actually reached business dispatch.
    pub native_events_dispatched: u64,
    pub ptt_mode: bool,
    /// True while a frontend capture session is suppressing runtime shortcuts.
    pub capturing: bool,
    pub diagnostics: Vec<String>,
}

// ==================== Internal state ====================

#[derive(Clone)]
struct FieldRuntime {
    field: ShortcutField,
    /// Persisted value (canonical for fields changed by the backend).
    value: String,
    normalized: NormalizedShortcut,
    template: String,
    /// Intended / confirmed registration for this field.
    registered: bool,
    uses_native_alt_right: bool,
    skip_reason: Option<String>,
}

impl FieldRuntime {
    fn empty(field: ShortcutField) -> Self {
        Self {
            field,
            value: String::new(),
            normalized: NormalizedShortcut::Empty,
            template: String::new(),
            registered: false,
            uses_native_alt_right: false,
            skip_reason: None,
        }
    }
}

#[derive(Default)]
struct VoiceSlot {
    current_task_id: Arc<Mutex<Option<u32>>>,
    recording_gen: Arc<AtomicU32>,
}

struct CaptureSession {
    token: u64,
    owner_window: String,
    field: ShortcutField,
    started_at: Instant,
}

struct Inner {
    registration_generation: u64,
    capture_generation: u64,
    capture: Option<CaptureSession>,
    next_capture_token: u64,
    /// Runtime registration is being switched: events must be dropped.
    switching: bool,
    runtime: [FieldRuntime; 4],
    pressed: [bool; 4],
    alt_right_backend: Option<native::AltRightBackend>,
    backend_kind: Option<BackendKind>,
    last_registration_errors: Vec<String>,
    native_events_seen: u64,
    native_events_dispatched: u64,
    voice_slots: [Arc<VoiceSlot>; 2],
}

#[derive(Debug, Clone, Copy)]
struct EventSnapshot {
    registration_generation: u64,
    capturing: bool,
}

enum DispatchMsg {
    Shortcut {
        field: ShortcutField,
        pressed: bool,
        snapshot: EventSnapshot,
    },
    AltRight {
        pressed: bool,
        snapshot: EventSnapshot,
    },
}

struct RegistrationResult {
    registered: [bool; 4],
    backend: Option<native::AltRightBackend>,
    backend_kind: Option<BackendKind>,
    errors: Vec<String>,
}

impl Default for RegistrationResult {
    fn default() -> Self {
        Self {
            registered: [false; 4],
            backend: None,
            backend_kind: None,
            errors: Vec::new(),
        }
    }
}

pub struct ShortcutManager {
    inner: Mutex<Inner>,
    dispatcher: Mutex<Option<mpsc::Sender<DispatchMsg>>>,
    update_lock: Mutex<()>,
}

fn is_windows() -> bool {
    cfg!(target_os = "windows")
}

impl ShortcutManager {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(Inner {
                registration_generation: 0,
                capture_generation: 0,
                capture: None,
                next_capture_token: 0,
                switching: false,
                runtime: std::array::from_fn(|i| FieldRuntime::empty(ShortcutField::ALL[i])),
                pressed: [false; 4],
                alt_right_backend: None,
                backend_kind: None,
                last_registration_errors: Vec::new(),
                native_events_seen: 0,
                native_events_dispatched: 0,
                voice_slots: [Arc::new(VoiceSlot::default()), Arc::new(VoiceSlot::default())],
            }),
            dispatcher: Mutex::new(None),
            update_lock: Mutex::new(()),
        }
    }

    // ---------- snapshot / dispatch plumbing ----------

    fn snapshot(&self) -> EventSnapshot {
        let mut inner = self.lock_inner();
        inner.expire_capture_if_stale();
        EventSnapshot {
            registration_generation: inner.registration_generation,
            capturing: inner.capture.is_some() || inner.switching,
        }
    }

    fn enqueue(&self, msg: DispatchMsg) {
        if let Ok(slot) = self.dispatcher.lock() {
            if let Some(tx) = slot.as_ref() {
                let _ = tx.send(msg);
            }
        }
    }

    fn ensure_dispatcher(&self, app: &AppHandle, recorder: &Arc<AudioRecorder>) {
        let mut slot = match self.dispatcher.lock() {
            Ok(slot) => slot,
            Err(poisoned) => poisoned.into_inner(),
        };
        if slot.is_some() {
            return;
        }
        let (tx, rx) = mpsc::channel::<DispatchMsg>();
        let app_thread = app.clone();
        let recorder_thread = recorder.clone();
        let spawned = std::thread::Builder::new()
            .name("byetype-shortcut-dispatch".to_string())
            .spawn(move || {
                while let Ok(msg) = rx.recv() {
                    let manager = app_thread.state::<ShortcutManager>();
                    manager.process_dispatch(msg, &app_thread, &recorder_thread);
                }
            });
        match spawned {
            Ok(_) => *slot = Some(tx),
            Err(e) => eprintln!("[shortcut] 无法创建调度线程: {e}"),
        }
    }

    fn on_shortcut_event(&self, field: ShortcutField, pressed: bool) {
        let snapshot = self.snapshot();
        self.enqueue(DispatchMsg::Shortcut {
            field,
            pressed,
            snapshot,
        });
    }

    fn process_dispatch(
        &self,
        msg: DispatchMsg,
        app: &AppHandle,
        recorder: &Arc<AudioRecorder>,
    ) {
        let (pressed, snapshot) = match &msg {
            DispatchMsg::Shortcut {
                pressed, snapshot, ..
            }
            | DispatchMsg::AltRight { pressed, snapshot } => (*pressed, *snapshot),
        };

        let mut inner = self.lock_inner();
        // 1. registration generation guard.
        if snapshot.registration_generation != inner.registration_generation {
            return;
        }
        // 2. capture snapshot guard: events produced during capture are dropped
        //    permanently and never replayed after the capture ends.
        if snapshot.capturing {
            return;
        }
        // 3. resolve field for native AltRight events.
        let field = match &msg {
            DispatchMsg::Shortcut { field, .. } => *field,
            DispatchMsg::AltRight { .. } => {
                match inner
                    .runtime
                    .iter()
                    .position(|f| f.registered && f.uses_native_alt_right)
                {
                    Some(index) => ShortcutField::ALL[index],
                    None => return,
                }
            }
        };
        let index = field.index();
        // 4. de-duplicate physical press/release pairs.
        if pressed {
            if inner.pressed[index] {
                return;
            }
            inner.pressed[index] = true;
        } else {
            if !inner.pressed[index] {
                // Isolated release: ignore.
                return;
            }
            inner.pressed[index] = false;
        }
        let runtime = inner.runtime[index].clone();
        drop(inner);

        if !runtime.registered {
            return;
        }

        {
            if matches!(msg, DispatchMsg::AltRight { .. }) {
                let mut inner = self.lock_inner();
                inner.native_events_dispatched = inner.native_events_dispatched.wrapping_add(1);
            }
        }

        match field {
            ShortcutField::Shortcut | ShortcutField::Shortcut2 => {
                let voice_index = if field == ShortcutField::Shortcut { 0 } else { 1 };
                let slot = self.lock_inner().voice_slots[voice_index].clone();
                let ptt = app.state::<ConfigManager>().get().general.ptt_mode;
                if ptt {
                    handle_ptt_event(app, pressed, recorder, &slot, &runtime.template);
                } else {
                    handle_toggle_event(app, pressed, recorder, &slot, &runtime.template);
                }
            }
            ShortcutField::ExtractShortcut | ShortcutField::ExtractShortcut2 => {
                if pressed {
                    crate::task::start_extraction(app, runtime.template.clone());
                }
            }
        }
    }

    // ---------- capture sessions ----------

    pub fn begin_capture(&self, owner_window: String, field: ShortcutField) -> u64 {
        let mut inner = self.lock_inner();
        inner.next_capture_token = inner.next_capture_token.wrapping_add(1).max(1);
        let token = inner.next_capture_token;
        inner.capture = Some(CaptureSession {
            token,
            owner_window,
            field,
            started_at: Instant::now(),
        });
        inner.capture_generation = inner.capture_generation.wrapping_add(1);
        token
    }

    pub fn end_capture(
        &self,
        token: u64,
        owner_window: &str,
        field: ShortcutField,
    ) -> Result<(), String> {
        self.close_capture(token, owner_window, field)
    }

    pub fn commit_capture(
        &self,
        token: u64,
        owner_window: &str,
        field: ShortcutField,
    ) -> Result<(), String> {
        self.close_capture(token, owner_window, field)
    }

    pub fn cancel_capture(
        &self,
        token: u64,
        owner_window: &str,
        field: ShortcutField,
    ) -> Result<(), String> {
        self.close_capture(token, owner_window, field)
    }

    fn close_capture(
        &self,
        token: u64,
        owner_window: &str,
        field: ShortcutField,
    ) -> Result<(), String> {
        let mut inner = self.lock_inner();
        let matches = inner
            .capture
            .as_ref()
            .map(|session| {
                session.token == token
                    && session.owner_window == owner_window
                    && session.field == field
            })
            .unwrap_or(false);
        if !matches {
            return Err("STALE_CAPTURE_SESSION: 迟到或不属于当前窗口的捕获会话已被忽略".to_string());
        }
        inner.capture = None;
        inner.capture_generation = inner.capture_generation.wrapping_add(1);
        Ok(())
    }

    pub fn status(&self) -> ShortcutStatus {
        let inner = self.lock_inner();
        let mut diagnostics: Vec<String> = inner
            .runtime
            .iter()
            .filter_map(|field| {
                field
                    .skip_reason
                    .as_ref()
                    .map(|reason| format!("{}: {}", field.field.label(), reason))
            })
            .collect();
        for error in &inner.last_registration_errors {
            let line = format!("注册警告: {error}");
            if !diagnostics.contains(&line) {
                diagnostics.push(line);
            }
        }
        if let Some(kind) = inner.backend_kind {
            diagnostics.push(format!("右 Alt 后端: {}", kind.as_str()));
        }
        ShortcutStatus {
            schema_mode: "supported".to_string(),
            editable: true,
            alt_right_backend: inner.backend_kind.map(|kind| kind.as_str().to_string()),
            native_events_seen: inner.native_events_seen,
            native_events_dispatched: inner.native_events_dispatched,
            ptt_mode: false,
            capturing: inner.capture.is_some() || inner.switching,
            diagnostics,
        }
    }

    fn lock_inner(&self) -> std::sync::MutexGuard<'_, Inner> {
        match self.inner.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        }
    }

    /// Append the current shortcut registration state to `<app_data_dir>/shortcut.log`
    /// so a failing native backend can be diagnosed without a debugger.
    fn log_registration(&self, app: &AppHandle, phase: &str) {
        let config = app.state::<ConfigManager>();
        let app_config = config.get();
        let migration_notes = config.migration_notes();
        let schema_mode = match config.schema_mode() {
            ShortcutSchemaMode::Supported => "supported",
            ShortcutSchemaMode::Future => "future",
        };
        let inner = self.lock_inner();
        let normalized: [NormalizedShortcut; 4] =
            std::array::from_fn(|i| inner.runtime[i].normalized.clone());
        let graph = build_conflict_graph(&normalized);
        let mut line = format!(
            "[{}] backend={:?} schema={} ptt={} switching={} native_seen={} native_dispatched={} errors={:?}\n",
            phase,
            inner.backend_kind,
            schema_mode,
            app_config.general.ptt_mode,
            inner.switching,
            inner.native_events_seen,
            inner.native_events_dispatched,
            inner.last_registration_errors
        );
        for note in &migration_notes {
            line.push_str(&format!("    config: {note}\n"));
        }
        let raw_general = config
            .get_raw()
            .get("general")
            .cloned()
            .unwrap_or(serde_json::Value::Null);
        line.push_str(&format!("    general: {raw_general}\n"));
        for pair in &graph.pairs {
            line.push_str(&format!(
                "    conflict: {} <-> {} ({:?})\n",
                pair.left.key(),
                pair.right.key(),
                pair.kind
            ));
        }
        for field in &inner.runtime {
            line.push_str(&format!(
                "    {} value={:?} normalized={:?} registered={} native={} skip={:?}\n",
                field.field.key(),
                field.value,
                field.normalized,
                field.registered,
                field.uses_native_alt_right,
                field.skip_reason
            ));
        }
        drop(inner);
        eprintln!("[shortcut] {line}");
        if let Ok(dir) = app.path().app_data_dir() {
            let _ = std::fs::create_dir_all(&dir);
            let path = dir.join("shortcut.log");
            // Bound the log so it cannot grow forever.
            if let Ok(meta) = std::fs::metadata(&path) {
                if meta.len() > 512 * 1024 {
                    let _ = std::fs::remove_file(&path);
                }
            }
            use std::io::Write;
            if let Ok(mut file) = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&path)
            {
                let _ = file.write_all(line.as_bytes());
            }
        }
    }

    // ---------- registration ----------

    fn teardown(&self, app: &AppHandle) {
        let backend = {
            let mut inner = self.lock_inner();
            inner.alt_right_backend.take()
        };
        if let Some(mut backend) = backend {
            backend.stop();
        }
        let _ = app.global_shortcut().unregister_all();
        let mut inner = self.lock_inner();
        inner.pressed = [false; 4];
    }

    fn perform_registration(
        &self,
        app: &AppHandle,
        fields: &[FieldRuntime; 4],
    ) -> RegistrationResult {
        let mut result = RegistrationResult::default();
        for (index, field) in fields.iter().enumerate() {
            if !field.registered || field.uses_native_alt_right {
                continue;
            }
            match register_tauri_shortcut(app, field) {
                Ok(()) => result.registered[index] = true,
                Err(error) => result.errors.push(error),
            }
        }

        if fields
            .iter()
            .any(|field| field.registered && field.uses_native_alt_right)
        {
            let sink = make_native_sink(app);
            match native::start_alt_right(sink) {
                Ok(backend) => {
                    result.backend_kind = Some(backend.kind());
                    for (index, field) in fields.iter().enumerate() {
                        if field.registered && field.uses_native_alt_right {
                            result.registered[index] = true;
                        }
                    }
                    result.backend = Some(backend);
                }
                Err(error) => result.errors.push(error),
            }
        }

        result
    }

    fn commit_runtime(
        &self,
        fields: [FieldRuntime; 4],
        backend: RegistrationResult,
    ) {
        let mut inner = self.lock_inner();
        let errors = backend.errors.clone();
        let mut runtime = fields;
        for (index, field) in runtime.iter_mut().enumerate() {
            field.registered = backend.registered[index];
            if !field.registered && field.uses_native_alt_right && field.skip_reason.is_none() {
                field.skip_reason = Some(if errors.is_empty() {
                    "ALT_RIGHT_BACKEND_FAILED: 右 Alt 后端未启动".to_string()
                } else {
                    errors.join("; ")
                });
            }
        }
        inner.runtime = runtime;
        inner.alt_right_backend = backend.backend;
        inner.backend_kind = backend.backend_kind;
        inner.last_registration_errors = errors;
        inner.pressed = [false; 4];
        inner.registration_generation = inner.registration_generation.wrapping_add(1);
        inner.switching = false;
    }

    // ---------- startup registration ----------

    fn register_from_config(&self, app: &AppHandle, recorder: &Arc<AudioRecorder>) -> Result<(), String> {
        self.ensure_dispatcher(app, recorder);
        let config = app.state::<ConfigManager>().get();
        let values = shortcut_values(&config);
        let templates = shortcut_templates(&config);
        let normalized = normalized_from_values(&values);
        let graph = build_conflict_graph(&normalized);
        let fields = build_fields(&values, &normalized, &graph, &templates);

        let _guard = self.lock_update();
        {
            let mut inner = self.lock_inner();
            inner.switching = true;
        }
        self.teardown(app);
        let result = self.perform_registration(app, &fields);
        for error in &result.errors {
            eprintln!("[shortcut] 启动注册警告: {error}");
        }
        self.commit_runtime(fields, result);
        self.log_registration(app, "startup");
        Ok(())
    }

    fn lock_update(&self) -> std::sync::MutexGuard<'_, ()> {
        match self.update_lock.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        }
    }

    // ---------- explicit field-level update ----------

    fn update_shortcuts(
        &self,
        app: &AppHandle,
        recorder: &Arc<AudioRecorder>,
        patch: &ShortcutPatch,
    ) -> Result<ShortcutUpdateResult, String> {
        self.ensure_dispatcher(app, recorder);
        let _guard = self.lock_update();

        let config_manager = app.state::<ConfigManager>();
        let current = config_manager.get();
        let mut values = shortcut_values(&current);
        let templates = shortcut_templates(&current);
        let mut changed = [false; 4];

        apply_patch_to_values(&mut values, &mut changed, patch);

        let normalized = normalized_from_values(&values);
        validate_candidates(&normalized, &changed).map_err(|error| {
            format!("{}: {}", error.code, error.message)
        })?;

        // On platforms without a native right-Alt backend an explicit AltRight
        // save must fail rather than silently persist an unregistered value.
        #[cfg(not(target_os = "windows"))]
        for index in 0..4 {
            if changed[index] && normalized[index].canonical() == Some(ALT_RIGHT) {
                return Err(
                    "ALT_RIGHT_BACKEND_FAILED: 当前平台不支持右 Alt 快捷键".to_string(),
                );
            }
        }

        let graph = build_conflict_graph(&normalized);
        let fields = build_fields(&values, &normalized, &graph, &templates);

        let old_fields = { self.lock_inner().runtime.clone() };

        {
            let mut inner = self.lock_inner();
            inner.switching = true;
        }
        self.teardown(app);
        let mut result = self.perform_registration(app, &fields);

        if !result.errors.is_empty() {
            // Stop any partially-created native backend before rolling back.
            if let Some(mut backend) = result.backend.take() {
                backend.stop();
            }
            // Roll back to the previously confirmed runtime.
            self.teardown(app);
            let rollback = self.perform_registration(app, &old_fields);
            {
                let mut inner = self.lock_inner();
                inner.alt_right_backend = rollback.backend;
                inner.backend_kind = rollback.backend_kind;
                inner.switching = false;
            }
            if !rollback.errors.is_empty() {
                eprintln!("[shortcut] 回滚失败: {:?}", rollback.errors);
                return Err(format!(
                    "SHORTCUT_ROLLBACK_FAILED: 新注册失败: {}; 回滚失败: {}",
                    result.errors.join("; "),
                    rollback.errors.join("; ")
                ));
            }
            return Err(format!(
                "SHORTCUT_REGISTER_FAILED: {}",
                result.errors.join("; ")
            ));
        }

        // Persist only the fields the user actually changed.
        let persist = build_persist_patch(&normalized, &changed);
        if !persist.is_empty() {
            if let Err(error) = config_manager.commit_shortcut_patch(&persist) {
                if let Some(mut backend) = result.backend.take() {
                    backend.stop();
                }
                self.teardown(app);
                let rollback = self.perform_registration(app, &old_fields);
                {
                    let mut inner = self.lock_inner();
                    inner.alt_right_backend = rollback.backend;
                    inner.backend_kind = rollback.backend_kind;
                    inner.switching = false;
                }
                if !rollback.errors.is_empty() {
                    return Err(format!(
                        "SHORTCUT_ROLLBACK_FAILED: 写盘失败后回滚失败: {}; {}",
                        error,
                        rollback.errors.join("; ")
                    ));
                }
                return Err(error);
            }
        }

        let backend_kind = result.backend_kind;
        self.commit_runtime(fields, result);
        self.log_registration(app, "update");

        Ok(build_update_result(&normalized, backend_kind, &persist))
    }
}

impl Inner {
    fn expire_capture_if_stale(&mut self) {
        if let Some(session) = &self.capture {
            if session.started_at.elapsed() > CAPTURE_MAX_AGE {
                self.capture = None;
                self.capture_generation = self.capture_generation.wrapping_add(1);
            }
        }
    }
}

// ==================== Public entry points ====================

/// Startup registration. Individual failures are tolerated and logged so a
/// single bad shortcut cannot take the whole application down.
pub fn register(app: &AppHandle, recorder: Arc<AudioRecorder>) -> Result<(), String> {
    let manager = app.state::<ShortcutManager>();
    manager.register_from_config(app, &recorder)
}

/// Explicit shortcut-domain update. All-or-nothing: any validation, OS
/// registration or persistence failure restores the previous runtime and the
/// previous on-disk values.
pub fn update_shortcuts(
    app: &AppHandle,
    recorder: Arc<AudioRecorder>,
    patch: ShortcutPatch,
) -> Result<ShortcutUpdateResult, String> {
    let manager = app.state::<ShortcutManager>();
    manager.update_shortcuts(app, &recorder, &patch)
}

/// Platform default for 语音输入 1, used by the "reset shortcuts" action.
pub fn default_primary_shortcut() -> String {
    crate::config::shortcut_config::platform_default(ShortcutField::Shortcut, is_windows())
        .to_string()
}

pub fn shortcut_status(app: &AppHandle) -> ShortcutStatus {
    let manager = app.state::<ShortcutManager>();
    let mode = app.state::<ConfigManager>().schema_mode();
    let ptt_mode = app.state::<ConfigManager>().get().general.ptt_mode;
    let mut status = manager.status();
    status.ptt_mode = ptt_mode;
    match mode {
        ShortcutSchemaMode::Supported => {}
        ShortcutSchemaMode::Future => {
            status.schema_mode = "future".to_string();
            // Our patch path is lossless (it only touches the four known
            // fields), so editing stays available; the UI still warns.
            status.editable = true;
            status
                .diagnostics
                .push("该配置由更高版本创建，快捷键修改将保留高版本字段".to_string());
        }
    }
    status
}

// ==================== Value helpers ====================

fn shortcut_values(config: &crate::config::types::AppConfig) -> [String; 4] {
    [
        config.general.shortcut.clone(),
        config.general.shortcut2.clone(),
        config.general.extract_shortcut.clone(),
        config.general.extract_shortcut2.clone(),
    ]
}

fn shortcut_templates(config: &crate::config::types::AppConfig) -> [String; 4] {
    [
        config.general.shortcut_template.clone(),
        config.general.shortcut2_template.clone(),
        config.general.extract_shortcut_template.clone(),
        config.general.extract_shortcut2_template.clone(),
    ]
}

fn normalized_from_values(values: &[String; 4]) -> [NormalizedShortcut; 4] {
    std::array::from_fn(|i| model::normalize(&values[i]))
}

fn apply_patch_to_values(
    values: &mut [String; 4],
    changed: &mut [bool; 4],
    patch: &ShortcutPatch,
) {
    let entries: [Option<&String>; 4] = [
        patch.shortcut.as_ref(),
        patch.shortcut2.as_ref(),
        patch.extract_shortcut.as_ref(),
        patch.extract_shortcut2.as_ref(),
    ];
    for (index, value) in entries.into_iter().enumerate() {
        if let Some(value) = value {
            values[index] = value.clone();
            changed[index] = true;
        }
    }
}

fn build_fields(
    values: &[String; 4],
    normalized: &[NormalizedShortcut; 4],
    graph: &model::ConflictGraph,
    templates: &[String; 4],
) -> [FieldRuntime; 4] {
    std::array::from_fn(|index| {
        let field = ShortcutField::ALL[index];
        let mut runtime = FieldRuntime::empty(field);
        runtime.value = values[index].clone();
        runtime.normalized = normalized[index].clone();
        runtime.template = templates[index].clone();

        match &normalized[index] {
            NormalizedShortcut::Empty => {}
            NormalizedShortcut::Invalid(raw) => {
                runtime.skip_reason = Some(format!("INVALID_SHORTCUT: {}", raw));
            }
            NormalizedShortcut::Valid(canonical) => {
                if graph.skipped[index] {
                    runtime.skip_reason = Some("SHORTCUT_CONFLICT".to_string());
                } else if is_alt_right(canonical) {
                    if is_windows() {
                        runtime.registered = true;
                        runtime.uses_native_alt_right = true;
                    } else {
                        runtime.skip_reason =
                            Some("ALT_RIGHT_BACKEND_FAILED: 当前平台不支持右 Alt".to_string());
                    }
                } else {
                    runtime.registered = true;
                }
            }
        }
        runtime
    })
}

fn build_persist_patch(
    normalized: &[NormalizedShortcut; 4],
    changed: &[bool; 4],
) -> Vec<(ShortcutField, String)> {
    let mut patch = Vec::new();
    for index in 0..4 {
        if !changed[index] {
            continue;
        }
        let value = match &normalized[index] {
            NormalizedShortcut::Empty => String::new(),
            NormalizedShortcut::Valid(canonical) => canonical.clone(),
            NormalizedShortcut::Invalid(raw) => raw.clone(),
        };
        patch.push((ShortcutField::ALL[index], value));
    }
    patch
}

fn build_update_result(
    normalized: &[NormalizedShortcut; 4],
    backend_kind: Option<BackendKind>,
    persist: &[(ShortcutField, String)],
) -> ShortcutUpdateResult {
    let confirmed = |index: usize| -> String {
        match &normalized[index] {
            NormalizedShortcut::Empty => String::new(),
            NormalizedShortcut::Valid(canonical) => canonical.clone(),
            NormalizedShortcut::Invalid(raw) => raw.clone(),
        }
    };
    let per_field = ShortcutField::ALL
        .iter()
        .enumerate()
        .map(|(index, field)| {
            let persisted = persist
                .iter()
                .find(|(patched, _)| patched == field)
                .map(|(_, value)| value.clone());
            FieldResult {
                field: field.key().to_string(),
                value: persisted.unwrap_or_else(|| confirmed(index)),
                registered: matches!(normalized[index], NormalizedShortcut::Valid(_)),
                error: None,
            }
        })
        .collect();

    ShortcutUpdateResult {
        shortcut: confirmed(0),
        shortcut2: confirmed(1),
        extract_shortcut: confirmed(2),
        extract_shortcut2: confirmed(3),
        alt_right_backend: backend_kind.map(|kind| kind.as_str().to_string()),
        per_field,
    }
}

fn make_native_sink(app: &AppHandle) -> native::EventSink {
    let app = app.clone();
    Arc::new(move |pressed: bool| {
        let manager = app.state::<ShortcutManager>();
        {
            let mut inner = manager.lock_inner();
            inner.native_events_seen = inner.native_events_seen.wrapping_add(1);
        }
        let snapshot = manager.snapshot();
        manager.enqueue(DispatchMsg::AltRight { pressed, snapshot });
    })
}

fn register_tauri_shortcut(app: &AppHandle, field: &FieldRuntime) -> Result<(), String> {
    let Some(key) = field.normalized.canonical().map(|value| value.to_string()) else {
        return Ok(());
    };
    let app_handle = app.clone();
    let field_id = field.field;
    app.global_shortcut()
        .on_shortcut(key.as_str(), move |_app, _shortcut, event| {
            let manager = app_handle.state::<ShortcutManager>();
            manager.on_shortcut_event(field_id, event.state == ShortcutState::Pressed);
        })
        .map_err(|error| {
            format!(
                "SHORTCUT_REGISTER_FAILED: {} ({})",
                field.value, error
            )
        })
}

// ==================== Voice / PTT business logic (unchanged) ====================

/// Toggle mode: press once to start, press again to stop + transcribe.
fn handle_toggle_event(
    app_handle: &AppHandle,
    pressed: bool,
    recorder: &Arc<AudioRecorder>,
    slot: &Arc<VoiceSlot>,
    tmpl: &str,
) {
    if !pressed {
        return;
    }

    if recorder.is_recording() {
        // CAS: claim the right to stop — advance gen to invalidate timer
        let gen = slot.recording_gen.load(Ordering::SeqCst);
        if slot
            .recording_gen
            .compare_exchange(gen, gen + 1, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            return; // Timer already stopped it
        }

        let task_id = slot.current_task_id.lock().unwrap().take();
        match recorder.stop() {
            Ok(base64_audio) => {
                update_tray_icon(app_handle, false);
                if let Some(tid) = task_id {
                    crate::task::process_recording(app_handle, tid, base64_audio, tmpl.to_string());
                }
            }
            Err(e) => {
                eprintln!("Stop recording error: {}", e);
                if let Some(tid) = task_id {
                    crate::task::cancel_recording(app_handle, tid);
                }
                update_tray_icon(app_handle, false);
                let _ = app_handle.emit(
                    "recording-error",
                    serde_json::json!({ "message": e }),
                );
            }
        }
    } else {
        start_voice_recording(app_handle, recorder, slot, tmpl);
    }
}

/// PTT mode: press to start, release to stop + transcribe (or cancel if too short).
fn handle_ptt_event(
    app_handle: &AppHandle,
    pressed: bool,
    recorder: &Arc<AudioRecorder>,
    slot: &Arc<VoiceSlot>,
    tmpl: &str,
) {
    if pressed {
        // Debounce: some platforms repeat Pressed events while held.
        if recorder.is_recording() {
            return;
        }
        start_voice_recording(app_handle, recorder, slot, tmpl);
    } else {
        // CAS: race against auto-timeout timer.
        let gen = slot.recording_gen.load(Ordering::SeqCst);
        if slot
            .recording_gen
            .compare_exchange(gen, gen + 1, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            return; // Auto-timeout already stopped it.
        }

        let task_id = slot.current_task_id.lock().unwrap().take();
        let elapsed = recorder
            .elapsed_since_start()
            .unwrap_or(Duration::ZERO);

        if elapsed < Duration::from_millis(PTT_MIN_DURATION_MS) {
            // Too short — discard recording, no transcription.
            let _ = recorder.cancel();
            update_tray_icon(app_handle, false);
            if let Some(tid) = task_id {
                crate::task::cancel_recording(app_handle, tid);
            }
        } else {
            match recorder.stop() {
                Ok(base64_audio) => {
                    update_tray_icon(app_handle, false);
                    if let Some(tid) = task_id {
                        crate::task::process_recording(
                            app_handle,
                            tid,
                            base64_audio,
                            tmpl.to_string(),
                        );
                    }
                }
                Err(e) => {
                    eprintln!("PTT stop recording error: {}", e);
                    if let Some(tid) = task_id {
                        crate::task::cancel_recording(app_handle, tid);
                    }
                    update_tray_icon(app_handle, false);
                    let _ = app_handle.emit(
                        "recording-error",
                        serde_json::json!({ "message": e }),
                    );
                }
            }
        }
    }
}

/// Shared start logic for both Toggle and PTT modes.
fn start_voice_recording(
    app_handle: &AppHandle,
    recorder: &Arc<AudioRecorder>,
    slot: &Arc<VoiceSlot>,
    tmpl: &str,
) {
    // Allocate the new generation BEFORE starting the recorder, so that any
    // Release event arriving while `recorder.start()` is still in progress
    // will see the up-to-date generation when it does its CAS.
    let gen = slot.recording_gen.fetch_add(1, Ordering::SeqCst) + 1;
    let mic = app_handle
        .state::<ConfigManager>()
        .get()
        .general
        .microphone
        .clone();
    let tid = match crate::task::start_recording(app_handle) {
        Some(tid) => tid,
        None => {
            // Max parallel reached — roll back the generation we allocated above
            // so a racing Release's CAS fails cleanly instead of consuming it.
            slot.recording_gen.fetch_add(1, Ordering::SeqCst);
            return;
        }
    };
    *slot.current_task_id.lock().unwrap() = Some(tid);
    match recorder.start(&mic) {
        Ok(()) => {
            update_tray_icon(app_handle, true);

            {
                let w_recorder = recorder.clone();
                let w_app = app_handle.clone();
                let w_gen = slot.recording_gen.clone();
                std::thread::spawn(move || {
                    // 兜底 3 秒：设备异常一直不送有效音频时也让气泡转红，不卡在准备中。
                    for _ in 0..READY_POLL_TICKS {
                        if w_gen.load(Ordering::SeqCst) != gen {
                            return; // 录音已结束，不要再改气泡
                        }
                        if w_recorder.audio_started() {
                            break;
                        }
                        std::thread::sleep(Duration::from_millis(READY_POLL_INTERVAL_MS));
                    }
                    if w_gen.load(Ordering::SeqCst) == gen {
                        let _ = crate::bubble::update(&w_app, tid, "recording");
                    }
                });
            }

            let max_secs = app_handle
                .state::<ConfigManager>()
                .get()
                .general
                .max_recording_seconds;
            if max_secs > 0 {
                let t_recorder = recorder.clone();
                let t_app = app_handle.clone();
                let t_task_id = slot.current_task_id.clone();
                let t_gen = slot.recording_gen.clone();
                let t_tmpl = tmpl.to_string();
                std::thread::spawn(move || {
                    std::thread::sleep(Duration::from_secs(max_secs as u64));
                    if t_gen
                        .compare_exchange(gen, gen + 1, Ordering::SeqCst, Ordering::SeqCst)
                        .is_ok()
                    {
                        let task_id = t_task_id.lock().unwrap().take();
                        match t_recorder.stop() {
                            Ok(base64_audio) => {
                                update_tray_icon(&t_app, false);
                                if let Some(tid) = task_id {
                                    crate::task::process_recording(
                                        &t_app,
                                        tid,
                                        base64_audio,
                                        t_tmpl.clone(),
                                    );
                                }
                            }
                            Err(e) => {
                                eprintln!("Auto-stop recording error: {}", e);
                                if let Some(tid) = task_id {
                                    crate::task::cancel_recording(&t_app, tid);
                                }
                                update_tray_icon(&t_app, false);
                                let _ = t_app.emit(
                                    "recording-error",
                                    serde_json::json!({ "message": e }),
                                );
                            }
                        }
                    }
                });
            }
        }
        Err(e) => {
            // Bump generation again to invalidate the just-allocated `gen`.
            slot.recording_gen.fetch_add(1, Ordering::SeqCst);
            *slot.current_task_id.lock().unwrap() = None;
            crate::task::cancel_recording(app_handle, tid);
            eprintln!("Start recording error: {}", e);
            let _ = app_handle.emit(
                "recording-error",
                serde_json::json!({ "message": e }),
            );
        }
    }
}

fn update_tray_icon(app: &AppHandle, is_recording: bool) {
    if let Some(tray) = app.tray_by_id("main-tray") {
        let icon_bytes: &[u8] = if is_recording {
            include_bytes!("../../icons/tray-recording.png")
        } else {
            include_bytes!("../../icons/tray-default.png")
        };
        if let Ok(icon) = tauri::image::Image::from_bytes(icon_bytes) {
            let _ = tray.set_icon(Some(icon));
        }
    }
}

// `ALT_RIGHT` is referenced by build_fields through model helpers; keep the
// symbol exported for tests and diagnostics.
#[allow(dead_code)]
pub const ALT_RIGHT_KEY: &str = ALT_RIGHT;

#[cfg(test)]
mod tests {
    use super::*;

    fn patch(values: [Option<&str>; 4]) -> ShortcutPatch {
        ShortcutPatch {
            shortcut: values[0].map(str::to_string),
            shortcut2: values[1].map(str::to_string),
            extract_shortcut: values[2].map(str::to_string),
            extract_shortcut2: values[3].map(str::to_string),
        }
    }

    #[test]
    fn build_persist_patch_normalizes_backspace_to_empty() {
        let mut values = [
            "Backspace".to_string(),
            String::new(),
            String::new(),
            String::new(),
        ];
        let mut changed = [false; 4];
        apply_patch_to_values(&mut values, &mut changed, &patch([Some("Backspace"), None, None, None]));
        let normalized = normalized_from_values(&values);
        let persist = build_persist_patch(&normalized, &changed);
        assert_eq!(persist, vec![(ShortcutField::Shortcut, String::new())]);
    }

    #[test]
    fn build_persist_patch_canonicalizes_valid_values() {
        let values = [
            "ctrl+shift+a".to_string(),
            String::new(),
            String::new(),
            String::new(),
        ];
        let normalized = normalized_from_values(&values);
        let persist = build_persist_patch(&normalized, &[true, false, false, false]);
        assert_eq!(
            persist,
            vec![(ShortcutField::Shortcut, "Ctrl+Shift+A".to_string())]
        );
    }

    #[test]
    fn build_fields_skips_conflicts_and_alt_on_other_platforms() {
        let values = [
            "F8".to_string(),
            "F8".to_string(),
            String::new(),
            String::new(),
        ];
        let normalized = normalized_from_values(&values);
        let graph = build_conflict_graph(&normalized);
        let templates = std::array::from_fn(|_| String::new());
        let fields = build_fields(&values, &normalized, &graph, &templates);
        assert!(!fields[0].registered);
        assert!(!fields[1].registered);
        assert!(fields[0].skip_reason.is_some());
    }

    #[test]
    fn patch_is_empty_detection() {
        assert!(ShortcutPatch::default().is_empty());
        assert!(!patch([Some(""), None, None, None]).is_empty());
    }
}
