use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

use crate::audio::recorder::AudioRecorder;
use crate::config::ConfigManager;

#[cfg(target_os = "windows")]
mod native;

#[cfg(target_os = "windows")]
static ALT_RIGHT_BACKEND: Mutex<Option<native::NativeBackend>> = Mutex::new(None);

const ALT_RIGHT: &str = "AltRight";

/// PTT 模式下，按住时间小于此阈值视为误触，丢弃录音。
const PTT_MIN_DURATION_MS: u64 = 300;

/// 等待麦克风送出首帧音频的轮询间隔。
const READY_POLL_INTERVAL_MS: u64 = 20;
/// 等待首帧音频的最大轮询次数（20ms × 150 = 3 秒兜底）。
const READY_POLL_TICKS: u32 = 150;

/// Register all 4 global shortcuts (2 voice + 2 image), each bound to its own template_id.
pub fn register(app: &AppHandle, recorder: Arc<AudioRecorder>) -> Result<(), String> {
    let cfg = app.state::<ConfigManager>().get();
    // 空值表示停用，不再给图片识别 1 偷偷补回 F6。
    let ek1 = cfg.general.extract_shortcut.clone();
    // 文本强化关闭时第二个语音快捷键不注册，也不参与冲突检测，
    // 避免它继续被触发或在未注册的情况下挡住保存。
    let second_voice_enabled =
        cfg.general.text_strengthening_enabled && !cfg.general.shortcut2.is_empty();
    let mut keys = vec![
        cfg.general.shortcut.clone(),
        ek1.clone(),
        cfg.general.extract_shortcut2.clone(),
    ];
    if second_voice_enabled {
        keys.push(cfg.general.shortcut2.clone());
    }

    // Conflict detection: registered shortcuts must be unique
    for i in 0..keys.len() {
        for j in (i + 1)..keys.len() {
            if !keys[i].is_empty() && keys[i] == keys[j] {
                return Err(format!("快捷键 '{}' 重复，请设置不同的快捷键", keys[i]));
            }
        }
    }

    #[cfg(target_os = "windows")]
    stop_alt_right();
    app.global_shortcut()
        .unregister_all()
        .map_err(|e| format!("Failed to unregister shortcuts: {}", e))?;

    #[cfg(target_os = "windows")]
    let mut alt_right_needed = false;

    // Voice shortcut 1
    if cfg.general.shortcut == ALT_RIGHT {
        #[cfg(target_os = "windows")]
        {
            alt_right_needed = true;
        }
        #[cfg(not(target_os = "windows"))]
        eprintln!("右 Alt 快捷键仅支持 Windows");
    } else if !cfg.general.shortcut.is_empty() {
        register_voice_shortcut(
            app,
            &cfg.general.shortcut,
            cfg.general.shortcut_template.clone(),
            recorder.clone(),
        )?;
    }
    // Voice shortcut 2 (仅文本强化开启时注册)
    if second_voice_enabled {
        if cfg.general.shortcut2 == ALT_RIGHT {
            #[cfg(target_os = "windows")]
            {
                alt_right_needed = true;
            }
            #[cfg(not(target_os = "windows"))]
            eprintln!("右 Alt 快捷键仅支持 Windows");
        } else {
            register_voice_shortcut(
                app,
                &cfg.general.shortcut2,
                cfg.general.shortcut2_template.clone(),
                recorder.clone(),
            )?;
        }
    }
    // Image shortcut 1：为空时不注册，保持停用状态。
    if ek1 == ALT_RIGHT {
        #[cfg(target_os = "windows")]
        {
            alt_right_needed = true;
        }
        #[cfg(not(target_os = "windows"))]
        eprintln!("右 Alt 快捷键仅支持 Windows");
    } else if !ek1.is_empty() {
        register_image_shortcut(app, &ek1, cfg.general.extract_shortcut_template.clone())?;
    }
    // Image shortcut 2
    if cfg.general.extract_shortcut2 == ALT_RIGHT {
        #[cfg(target_os = "windows")]
        {
            alt_right_needed = true;
        }
        #[cfg(not(target_os = "windows"))]
        eprintln!("右 Alt 快捷键仅支持 Windows");
    } else if !cfg.general.extract_shortcut2.is_empty() {
        register_image_shortcut(
            app,
            &cfg.general.extract_shortcut2,
            cfg.general.extract_shortcut2_template.clone(),
        )?;
    }

    #[cfg(target_os = "windows")]
    if alt_right_needed {
        start_alt_right(app, recorder)?;
    }

    Ok(())
}

#[cfg(target_os = "windows")]
fn stop_alt_right() {
    let backend = ALT_RIGHT_BACKEND
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .take();
    drop(backend);
}

/// 原生监听只负责把右 Alt 的按下/松开送到工作线程，避免在系统键盘钩子里
/// 执行录音、网络请求等耗时逻辑。配置在事件发生时读取，与现有快捷键行为一致。
#[cfg(target_os = "windows")]
fn start_alt_right(app: &AppHandle, recorder: Arc<AudioRecorder>) -> Result<(), String> {
    let (event_tx, event_rx) = std::sync::mpsc::channel::<bool>();
    let worker_app = app.clone();
    let current_task_id: Arc<Mutex<Option<u32>>> = Arc::new(Mutex::new(None));
    let recording_gen: Arc<AtomicU32> = Arc::new(AtomicU32::new(0));

    std::thread::Builder::new()
        .name("byetype-altright-events".to_string())
        .spawn(move || {
            let mut is_down = false;
            while let Ok(pressed) = event_rx.recv() {
                // Windows 会在按住键时重复发送按下事件，只响应真实的状态变化。
                if pressed == is_down {
                    continue;
                }
                is_down = pressed;

                let config = worker_app.state::<ConfigManager>().get();
                let general = &config.general;
                let voice_template = if general.shortcut == ALT_RIGHT {
                    Some(general.shortcut_template.clone())
                } else if general.text_strengthening_enabled && general.shortcut2 == ALT_RIGHT {
                    Some(general.shortcut2_template.clone())
                } else {
                    None
                };

                if let Some(template) = voice_template {
                    let state = if pressed {
                        ShortcutState::Pressed
                    } else {
                        ShortcutState::Released
                    };
                    if general.ptt_mode {
                        handle_ptt_event(
                            &worker_app,
                            state,
                            &recorder,
                            &current_task_id,
                            &recording_gen,
                            &template,
                        );
                    } else {
                        handle_toggle_event(
                            &worker_app,
                            state,
                            &recorder,
                            &current_task_id,
                            &recording_gen,
                            &template,
                        );
                    }
                    continue;
                }

                if !pressed {
                    continue;
                }
                let image_template = if general.extract_shortcut == ALT_RIGHT {
                    Some(general.extract_shortcut_template.clone())
                } else if general.extract_shortcut2 == ALT_RIGHT {
                    Some(general.extract_shortcut2_template.clone())
                } else {
                    None
                };
                if let Some(template) = image_template {
                    crate::task::start_extraction(&worker_app, template);
                }
            }
        })
        .map_err(|e| format!("无法创建右 Alt 事件线程: {e}"))?;

    let sink: native::EventSink = Arc::new(move |pressed| {
        let _ = event_tx.send(pressed);
    });
    let backend = native::NativeBackend::start(sink)?;
    *ALT_RIGHT_BACKEND.lock().unwrap_or_else(|e| e.into_inner()) = Some(backend);
    Ok(())
}

/// Register a single voice (recording) shortcut bound to the given template_id.
/// Supports both Toggle mode (default) and Push-to-Talk mode based on config.general.ptt_mode
/// at event time — switching mode does NOT require re-registering the shortcut.
fn register_voice_shortcut(
    app: &AppHandle,
    shortcut_key: &str,
    template_id: String,
    recorder: Arc<AudioRecorder>,
) -> Result<(), String> {
    let app_handle = app.clone();
    let tmpl = template_id;
    // Track the current recording's task_id between start and stop
    let current_task_id: Arc<Mutex<Option<u32>>> = Arc::new(Mutex::new(None));
    let recording_gen: Arc<AtomicU32> = Arc::new(AtomicU32::new(0));

    app.global_shortcut()
        .on_shortcut(shortcut_key, move |_app, _shortcut, event| {
            let ptt = app_handle.state::<ConfigManager>().get().general.ptt_mode;

            if ptt {
                handle_ptt_event(
                    &app_handle,
                    event.state,
                    &recorder,
                    &current_task_id,
                    &recording_gen,
                    &tmpl,
                );
            } else {
                handle_toggle_event(
                    &app_handle,
                    event.state,
                    &recorder,
                    &current_task_id,
                    &recording_gen,
                    &tmpl,
                );
            }
        })
        .map_err(|e| format!("Failed to register voice shortcut '{}': {}", shortcut_key, e))?;

    Ok(())
}

/// Toggle mode: press once to start, press again to stop + transcribe.
/// Original behavior — kept identical to pre-PTT code path.
fn handle_toggle_event(
    app_handle: &AppHandle,
    state: ShortcutState,
    recorder: &Arc<AudioRecorder>,
    current_task_id: &Arc<Mutex<Option<u32>>>,
    recording_gen: &Arc<AtomicU32>,
    tmpl: &str,
) {
    if state != ShortcutState::Pressed {
        return;
    }

    if recorder.is_recording() {
        // CAS: claim the right to stop — advance gen to invalidate timer
        let gen = recording_gen.load(Ordering::SeqCst);
        if recording_gen.compare_exchange(gen, gen + 1, Ordering::SeqCst, Ordering::SeqCst).is_err() {
            return; // Timer already stopped it
        }

        let task_id = current_task_id.lock().unwrap().take();
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
                let _ = app_handle.emit("recording-error", serde_json::json!({
                    "message": e
                }));
            }
        }
    } else {
        start_voice_recording(app_handle, recorder, current_task_id, recording_gen, tmpl);
    }
}

/// PTT mode: press to start, release to stop + transcribe (or cancel if too short).
fn handle_ptt_event(
    app_handle: &AppHandle,
    state: ShortcutState,
    recorder: &Arc<AudioRecorder>,
    current_task_id: &Arc<Mutex<Option<u32>>>,
    recording_gen: &Arc<AtomicU32>,
    tmpl: &str,
) {
    match state {
        ShortcutState::Pressed => {
            // Debounce: some platforms repeat Pressed events while held.
            if recorder.is_recording() {
                return;
            }
            start_voice_recording(app_handle, recorder, current_task_id, recording_gen, tmpl);
        }
        ShortcutState::Released => {
            // CAS: race against auto-timeout timer.
            let gen = recording_gen.load(Ordering::SeqCst);
            if recording_gen.compare_exchange(gen, gen + 1, Ordering::SeqCst, Ordering::SeqCst).is_err() {
                return; // Auto-timeout already stopped it.
            }

            let task_id = current_task_id.lock().unwrap().take();
            let elapsed = recorder.elapsed_since_start().unwrap_or(Duration::ZERO);

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
                            crate::task::process_recording(app_handle, tid, base64_audio, tmpl.to_string());
                        }
                    }
                    Err(e) => {
                        eprintln!("PTT stop recording error: {}", e);
                        if let Some(tid) = task_id {
                            crate::task::cancel_recording(app_handle, tid);
                        }
                        update_tray_icon(app_handle, false);
                        let _ = app_handle.emit("recording-error", serde_json::json!({
                            "message": e
                        }));
                    }
                }
            }
        }
    }
}

/// Shared start logic for both Toggle and PTT modes.
/// Starts the recorder, allocates a task_id, shows the bubble, and spawns the
/// max-duration auto-stop timer (which races with manual stop via `recording_gen` CAS).
fn start_voice_recording(
    app_handle: &AppHandle,
    recorder: &Arc<AudioRecorder>,
    current_task_id: &Arc<Mutex<Option<u32>>>,
    recording_gen: &Arc<AtomicU32>,
    tmpl: &str,
) {
    // Allocate the new generation BEFORE starting the recorder, so that any
    // Release event arriving while `recorder.start()` is still in progress
    // will see the up-to-date generation when it does its CAS — preventing
    // the race where Release loads a stale `gen` (the value from before this
    // Press) and its CAS succeeds with the wrong version, dropping the recording.
    let gen = recording_gen.fetch_add(1, Ordering::SeqCst) + 1;
    let mic = app_handle.state::<ConfigManager>().get().general.microphone.clone();
    // Allocate the task_id and publish it BEFORE starting the recorder, so that
    // any Release event (or auto-stop timer) arriving between recorder.start()
    // returning Ok and the task_id being stored will still find a task_id in
    // place — preventing the race where stop succeeds but process_recording is
    // skipped because task_id was still None, silently dropping the recording.
    let tid = match crate::task::start_recording(app_handle) {
        Some(tid) => tid,
        None => {
            // Max parallel reached — roll back the generation we allocated above
            // so a racing Release's CAS fails cleanly instead of consuming it.
            recording_gen.fetch_add(1, Ordering::SeqCst);
            return;
        }
    };
    *current_task_id.lock().unwrap() = Some(tid);
    match recorder.start(&mic) {
        Ok(()) => {
            update_tray_icon(app_handle, true);

            // 蓝牙耳机要先完成 HFP 握手才会送出第一帧音频，这段时间麦克风
            // 其实没在采集。气泡先停在 preparing，收到首个回调后再转 recording，
            // 用户看到红点再开口，避免开头几秒白说。有线/内置麦克风几乎瞬间
            // 回调，观感上和以前一样。
            {
                let w_recorder = recorder.clone();
                let w_app = app_handle.clone();
                let w_gen = recording_gen.clone();
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

            let max_secs = app_handle.state::<ConfigManager>().get().general.max_recording_seconds;
            if max_secs > 0 {
                let t_recorder = recorder.clone();
                let t_app = app_handle.clone();
                let t_task_id = current_task_id.clone();
                let t_gen = recording_gen.clone();
                let t_tmpl = tmpl.to_string();
                std::thread::spawn(move || {
                    std::thread::sleep(Duration::from_secs(max_secs as u64));
                    if t_gen.compare_exchange(gen, gen + 1, Ordering::SeqCst, Ordering::SeqCst).is_ok() {
                        let task_id = t_task_id.lock().unwrap().take();
                        match t_recorder.stop() {
                            Ok(base64_audio) => {
                                update_tray_icon(&t_app, false);
                                if let Some(tid) = task_id {
                                    crate::task::process_recording(&t_app, tid, base64_audio, t_tmpl.clone());
                                }
                            }
                            Err(e) => {
                                eprintln!("Auto-stop recording error: {}", e);
                                if let Some(tid) = task_id {
                                    crate::task::cancel_recording(&t_app, tid);
                                }
                                update_tray_icon(&t_app, false);
                                let _ = t_app.emit("recording-error", serde_json::json!({
                                    "message": e
                                }));
                            }
                        }
                    }
                });
            }
        }
        Err(e) => {
            // Bump generation again to invalidate the just-allocated `gen` —
            // any in-flight Release racing with this failed Press will then
            // see a fresh value and its CAS will fail cleanly.
            recording_gen.fetch_add(1, Ordering::SeqCst);
            // Clean up the task_id we pre-allocated: clear it from
            // current_task_id so no later path mistakes it for a live
            // recording, and cancel the task to release its slot/bubble.
            *current_task_id.lock().unwrap() = None;
            crate::task::cancel_recording(app_handle, tid);
            eprintln!("Start recording error: {}", e);
            let _ = app_handle.emit("recording-error", serde_json::json!({
                "message": e
            }));
        }
    }
}

/// Register a single image (screenshot + OCR extraction) shortcut bound to the given template_id.
fn register_image_shortcut(
    app: &AppHandle,
    shortcut_key: &str,
    template_id: String,
) -> Result<(), String> {
    let extract_app = app.clone();
    let tmpl = template_id;
    app.global_shortcut()
        .on_shortcut(shortcut_key, move |_app, _shortcut, event| {
            if event.state != ShortcutState::Pressed { return; }
            crate::task::start_extraction(&extract_app, tmpl.clone());
        })
        .map_err(|e| format!("Failed to register image shortcut '{}': {}", shortcut_key, e))?;
    Ok(())
}

fn update_tray_icon(app: &AppHandle, is_recording: bool) {
    if let Some(tray) = app.tray_by_id("main-tray") {
        let icon_bytes: &[u8] = if is_recording {
            include_bytes!("../icons/tray-recording.png")
        } else {
            include_bytes!("../icons/tray-default.png")
        };
        if let Ok(icon) = tauri::image::Image::from_bytes(icon_bytes) {
            let _ = tray.set_icon(Some(icon));
        }
    }
}
