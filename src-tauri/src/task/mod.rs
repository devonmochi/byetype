pub mod history;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tokio_util::sync::CancellationToken;
use tauri::{AppHandle, Emitter, Manager};
use crate::config::ConfigManager;
use crate::ai;
use history::{HistoryManager, HistoryRecord};

pub struct TaskManager {
    task_counter: u32,
    active_count: u32,
    history: HistoryManager,
    prompts_dir: PathBuf,
    cancel_tokens: HashMap<u32, (CancellationToken, Option<u64>)>,
}

impl TaskManager {
    pub fn new(data_dir: &std::path::Path, prompts_dir: PathBuf) -> Self {
        let mut history = HistoryManager::new(data_dir);
        if let Err(e) = history.init() {
            eprintln!("[TaskManager] History init error: {}", e);
        }
        Self { task_counter: 0, active_count: 0, history, prompts_dir, cancel_tokens: HashMap::new() }
    }

    pub fn get_records(&self) -> &[HistoryRecord] {
        self.history.get_records()
    }

    pub fn get_audio_base64(&self, record_id: u64) -> Option<String> {
        self.history.get_audio_base64(record_id)
    }

    pub fn clear_history(&mut self) -> Result<(), String> {
        self.history.clear()
    }
}

pub type SharedTaskManager = Arc<Mutex<TaskManager>>;

/// Called from shortcut.rs when recording STARTS.
/// Allocates a task_id, shows bubble with "recording" status, returns the task_id.
pub fn start_recording(app: &AppHandle) -> Option<u32> {
    let config = app.state::<ConfigManager>().get();
    let task_id = {
        let state = app.state::<SharedTaskManager>();
        let mut mgr = state.lock().unwrap();
        if mgr.active_count >= config.advanced.max_parallel {
            eprintln!(
                "[TaskManager] Max parallel tasks reached ({}), cannot start",
                config.advanced.max_parallel
            );
            return None;
        }
        if mgr.active_count == 0 {
            mgr.task_counter = 0;
        }
        mgr.task_counter += 1;
        mgr.active_count += 1;
        let token = CancellationToken::new();
        let id = mgr.task_counter;
        mgr.cancel_tokens.insert(id, (token, None));
        id
    };

    if let Err(e) = crate::bubble::show(app, task_id) {
        eprintln!("[TaskManager] Failed to show bubble: {}", e);
    }
    Some(task_id)
}

/// Called from shortcut.rs when recording STOPS successfully.
pub fn process_recording(app: &AppHandle, task_id: u32, audio_base64: String) {
    let token = {
        let state = app.state::<SharedTaskManager>();
        let mgr = state.lock().unwrap();
        mgr.cancel_tokens.get(&task_id).map(|(t, _)| t.clone())
    };
    let token = match token {
        Some(t) => t,
        None => return,
    };
    let app_handle = app.clone();
    tauri::async_runtime::spawn(async move {
        run_pipeline(&app_handle, task_id, audio_base64, None, token).await;
    });
}

/// Called when recording fails or is cancelled. Cleans up the pre-allocated task.
pub fn cancel_recording(app: &AppHandle, task_id: u32) {
    let _ = crate::bubble::update(app, task_id, "failed");
    let _ = crate::bubble::hide(app, task_id, 2000);
    let state = app.state::<SharedTaskManager>();
    let mut mgr = state.lock().unwrap();
    mgr.cancel_tokens.remove(&task_id);
    mgr.active_count = mgr.active_count.saturating_sub(1);
}

/// Retry a previously failed record.
pub fn retry_record(app: &AppHandle, record_id: u64) {
    let config = app.state::<ConfigManager>().get();

    let (task_id, audio_base64, token) = {
        let state = app.state::<SharedTaskManager>();
        let mut mgr = state.lock().unwrap();

        let audio = match mgr.get_audio_base64(record_id) {
            Some(a) => a,
            None => {
                eprintln!("[TaskManager] No audio found for record {}", record_id);
                return;
            }
        };

        if mgr.active_count >= config.advanced.max_parallel {
            eprintln!("[TaskManager] Max parallel tasks reached, cannot retry");
            return;
        }
        if mgr.active_count == 0 {
            mgr.task_counter = 0;
        }
        mgr.task_counter += 1;
        mgr.active_count += 1;
        let token = CancellationToken::new();
        let id = mgr.task_counter;
        mgr.cancel_tokens.insert(id, (token.clone(), Some(record_id)));
        (id, audio, token)
    };

    if let Err(e) = crate::bubble::show(app, task_id) {
        eprintln!("[TaskManager] Failed to show bubble: {}", e);
    }

    let app_handle = app.clone();
    tauri::async_runtime::spawn(async move {
        run_pipeline(&app_handle, task_id, audio_base64, Some(record_id), token).await;
    });
}

fn build_client(proxy_url: &str) -> Result<reqwest::Client, String> {
    if proxy_url.is_empty() {
        reqwest::Client::builder()
            .build()
            .map_err(|e| format!("Failed to build HTTP client: {}", e))
    } else {
        let proxy = reqwest::Proxy::all(proxy_url)
            .map_err(|e| format!("Invalid proxy URL: {}", e))?;
        reqwest::Client::builder()
            .proxy(proxy)
            .build()
            .map_err(|e| format!("Failed to build proxy client: {}", e))
    }
}

async fn run_pipeline(
    app: &AppHandle,
    task_id: u32,
    audio_base64: String,
    retry_record_id: Option<u64>,
    token: CancellationToken,
) {
    // Get config snapshot and prompts_dir - release lock before any .await
    let (config, prompts_dir) = {
        let state = app.state::<SharedTaskManager>();
        let mgr = state.lock().unwrap();
        (app.state::<ConfigManager>().get(), mgr.prompts_dir.clone())
    };

    let max_retries = config.advanced.max_retries;
    let transcribe_timeout = config.advanced.transcribe_timeout;
    let optimize_timeout = config.advanced.optimize_timeout;

    // Build HTTP client (once per pipeline run)
    let client = match build_client(&config.advanced.proxy_url) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("[TaskManager] Failed to build client: {}", e);
            finish_pipeline(
                app, task_id, retry_record_id, &audio_base64,
                None, None, "failed", Some(e),
            );
            return;
        }
    };

    // Phase 1: Transcribe
    let _ = crate::bubble::update(app, task_id, "transcribing");
    if let Some(rid) = retry_record_id {
        let _ = app.emit("retry-status", serde_json::json!({ "recordId": rid, "status": "transcribing" }));
    }

    let transcribe_result = {
        let client = client.clone();
        let audio = audio_base64.clone();
        let cfg = config.clone();
        let pd = prompts_dir.clone();
        tokio::select! {
            result = ai::retry::with_retry(
                || {
                    let client = client.clone();
                    let audio = audio.clone();
                    let cfg = cfg.clone();
                    let pd = pd.clone();
                    async move {
                        ai::transcribe(&client, &audio, &cfg, &pd).await
                    }
                },
                max_retries,
                transcribe_timeout,
            ) => result,
            _ = token.cancelled() => {
                eprintln!("[TaskManager] Task {} cancelled during transcription", task_id);
                return;
            }
        }
    };

    let transcribe_text: String = match transcribe_result {
        Ok(text) => text,
        Err(e) => {
            eprintln!("[TaskManager] Transcription failed: {}", e);
            finish_pipeline(
                app, task_id, retry_record_id, &audio_base64,
                None, None, "failed", Some(e),
            );
            return;
        }
    };

    // Phase 2: Optimize (if enabled)
    let final_text: String;
    let optimize_text: Option<String>;

    if config.optimize.enabled {
        let _ = crate::bubble::update(app, task_id, "optimizing");
        if let Some(rid) = retry_record_id {
            let _ = app.emit("retry-status", serde_json::json!({ "recordId": rid, "status": "optimizing" }));
        }

        let optimize_result = {
            let client = client.clone();
            let txt = transcribe_text.clone();
            let cfg = config.clone();
            let pd = prompts_dir.clone();
            tokio::select! {
                result = ai::retry::with_retry(
                    || {
                        let client = client.clone();
                        let txt = txt.clone();
                        let cfg = cfg.clone();
                        let pd = pd.clone();
                        async move {
                            ai::optimize(&client, &txt, &cfg, &pd).await
                        }
                    },
                    max_retries,
                    optimize_timeout,
                ) => result,
                _ = token.cancelled() => {
                    eprintln!("[TaskManager] Task {} cancelled during optimization", task_id);
                    return;
                }
            }
        };

        match optimize_result {
            Ok(text) => {
                optimize_text = Some(text.clone());
                final_text = text;
            }
            Err(e) => {
                eprintln!("[TaskManager] Optimization failed: {}", e);
                finish_pipeline(
                    app, task_id, retry_record_id, &audio_base64,
                    Some(transcribe_text), None, "failed", Some(e),
                );
                return;
            }
        }
    } else {
        optimize_text = None;
        final_text = transcribe_text.clone();
    }

    // Phase 3: Paste result
    if let Err(e) = crate::clipboard::paste_text(&final_text) {
        eprintln!("[TaskManager] Paste failed: {}", e);
    }

    // Success
    finish_pipeline(
        app, task_id, retry_record_id, &audio_base64,
        Some(transcribe_text), optimize_text, "completed", None,
    );
}

fn finish_pipeline(
    app: &AppHandle,
    task_id: u32,
    retry_record_id: Option<u64>,
    audio_base64: &str,
    transcribe_text: Option<String>,
    optimize_text: Option<String>,
    status: &str,
    error_message: Option<String>,
) {
    let state = app.state::<SharedTaskManager>();
    let mut mgr = state.lock().unwrap();

    // Atomic guard: whoever removes the token first owns cleanup
    match mgr.cancel_tokens.remove(&task_id) {
        Some((token, _)) if token.is_cancelled() => return,
        Some(_) => { /* normal completion, proceed */ }
        None => return,
    }

    // Update bubble (after guard)
    let bubble_delay = if status == "completed" { 1500 } else { 3000 };
    let _ = crate::bubble::update(app, task_id, status);
    let _ = crate::bubble::hide(app, task_id, bubble_delay);

    // Update history
    if let Some(rid) = retry_record_id {
        if let Err(e) = mgr.history.update_record(
            rid,
            transcribe_text,
            optimize_text,
            status,
            error_message,
        ) {
            eprintln!("[TaskManager] Failed to update history record: {}", e);
        }
    } else {
        if let Err(e) = mgr.history.add_record(
            Some(audio_base64),
            transcribe_text,
            optimize_text,
            status,
            error_message,
        ) {
            eprintln!("[TaskManager] Failed to add history record: {}", e);
        }
    }

    // Emit updated records
    let records = mgr.history.get_records();
    let json_records = serde_json::to_value(records).unwrap_or(serde_json::json!([]));
    let _ = app.emit("history-updated", json_records);

    if let Some(rid) = retry_record_id {
        let _ = app.emit("retry-status", serde_json::json!({
            "recordId": rid,
            "status": status
        }));
    }

    // Decrement active count
    mgr.active_count = mgr.active_count.saturating_sub(1);
}
