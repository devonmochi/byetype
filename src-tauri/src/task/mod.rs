pub mod history;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tokio_util::sync::CancellationToken;
use tauri::{AppHandle, Emitter, Manager};
use base64::Engine as _;
use crate::config::ConfigManager;
use crate::ai;
use history::{HistoryManager, HistoryRecord};

/// Screenshot selection coordinates from the overlay window
#[derive(serde::Deserialize, Clone)]
pub struct ScreenshotCrop {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

/// Oneshot sender for screenshot crop result. None = user cancelled.
pub type ScreenshotSender = Arc<Mutex<Option<tokio::sync::oneshot::Sender<Option<ScreenshotCrop>>>>>;

/// Stores the full-screen screenshot base64 for the overlay window to fetch.
pub type ScreenshotImageState = Arc<Mutex<Option<String>>>;

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

/// Cancel an in-progress transcription/optimization task.
pub fn cancel_task(app: &AppHandle, task_id: u32) {
    let state = app.state::<SharedTaskManager>();
    let mut mgr = state.lock().unwrap();

    // Take token — whoever removes it first owns cleanup
    let (token, retry_record_id) = match mgr.cancel_tokens.remove(&task_id) {
        Some(entry) => entry,
        None => return, // Task already finished, nothing to do
    };

    // Signal cancellation to run_pipeline
    token.cancel();

    // Immediately hide bubble
    let _ = crate::bubble::update(app, task_id, "failed");
    let _ = crate::bubble::hide(app, task_id, 0);

    // Write history record
    if let Some(rid) = retry_record_id {
        // Retry task: update the original record
        if let Err(e) = mgr.history.update_record(
            rid,
            None,
            None,
            "cancelled",
            Some("用户取消".to_string()),
        ) {
            eprintln!("[TaskManager] Failed to update cancelled record: {}", e);
        }
        // Notify frontend about retry status
        let _ = app.emit("retry-status", serde_json::json!({
            "recordId": rid,
            "status": "cancelled"
        }));
    } else {
        // New task: create a new cancelled record
        if let Err(e) = mgr.history.add_record(
            None,
            None,
            None,
            "cancelled",
            Some("用户取消".to_string()),
        ) {
            eprintln!("[TaskManager] Failed to add cancelled record: {}", e);
        }
    }

    // Emit updated history
    let records = mgr.history.get_records();
    let json_records = serde_json::to_value(records).unwrap_or(serde_json::json!([]));
    let _ = app.emit("history-updated", json_records);

    // Decrement active count
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
                |_attempt| {
                    let _ = crate::bubble::update(app, task_id, "retrying");
                    if let Some(rid) = retry_record_id {
                        let _ = app.emit("retry-status", serde_json::json!({
                            "recordId": rid, "status": "retrying"
                        }));
                    }
                },
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
                    |_attempt| {
                        let _ = crate::bubble::update(app, task_id, "retrying");
                        if let Some(rid) = retry_record_id {
                            let _ = app.emit("retry-status", serde_json::json!({
                                "recordId": rid, "status": "retrying"
                            }));
                        }
                    },
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

/// Called from shortcut.rs when user triggers the extract shortcut.
/// Takes a screenshot via interactive selection, OCRs it, and copies text to clipboard.
pub fn start_extraction(app: &AppHandle) -> Option<u32> {
    let config = app.state::<ConfigManager>().get();
    let task_id = {
        let state = app.state::<SharedTaskManager>();
        let mut mgr = state.lock().unwrap();
        if mgr.active_count >= config.advanced.max_parallel {
            eprintln!(
                "[TaskManager] Max parallel tasks reached ({}), cannot start extraction",
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

    let app_handle = app.clone();
    tauri::async_runtime::spawn(async move {
        run_extract_pipeline(&app_handle, task_id).await;
    });
    Some(task_id)
}

async fn run_extract_pipeline(app: &AppHandle, task_id: u32) {
    // Phase 0: Interactive screenshot capture (platform-specific)
    let image_base64 = match capture_screenshot(app, task_id).await {
        Some(b64) => b64,
        None => return, // User cancelled or capture failed
    };

    // Show bubble with extracting status
    if let Err(e) = crate::bubble::show(app, task_id) {
        eprintln!("[TaskManager] Failed to show bubble: {}", e);
    }
    let _ = crate::bubble::update(app, task_id, "extracting");

    // Get config snapshot and prompts_dir
    let (config, prompts_dir, token) = {
        let state = app.state::<SharedTaskManager>();
        let mgr = state.lock().unwrap();
        let tok = mgr.cancel_tokens.get(&task_id).map(|(t, _)| t.clone());
        (app.state::<ConfigManager>().get(), mgr.prompts_dir.clone(), tok)
    };

    let token = match token {
        Some(t) => t,
        None => return,
    };

    let max_retries = config.advanced.max_retries;
    let extract_timeout = config.advanced.transcribe_timeout; // reuse transcribe timeout for extract

    // Build HTTP client
    let client = match build_client(&config.advanced.proxy_url) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("[TaskManager] Failed to build client: {}", e);
            finish_extract_pipeline(
                app, task_id, Some(&image_base64), None, "failed", Some(e),
            );
            return;
        }
    };

    // Phase 1: Extract text from screenshot via AI
    let extract_result = {
        let client = client.clone();
        let img = image_base64.clone();
        let cfg = config.clone();
        let pd = prompts_dir.clone();
        tokio::select! {
            result = ai::retry::with_retry(
                || {
                    let client = client.clone();
                    let img = img.clone();
                    let cfg = cfg.clone();
                    let pd = pd.clone();
                    async move {
                        ai::extract_text(&client, &img, &cfg, &pd).await
                    }
                },
                max_retries,
                extract_timeout,
                |_attempt| {
                    let _ = crate::bubble::update(app, task_id, "retrying");
                },
            ) => result,
            _ = token.cancelled() => {
                eprintln!("[TaskManager] Task {} cancelled during extraction", task_id);
                return;
            }
        }
    };

    match extract_result {
        Ok(text) => {
            // Copy text to clipboard (without pasting)
            if let Ok(mut clipboard) = arboard::Clipboard::new() {
                let _ = clipboard.set_text(&text);
            }

            // Show preview window
            if let Err(e) = crate::preview::show(app, &text) {
                eprintln!("[TaskManager] Failed to show preview: {}", e);
            }

            finish_extract_pipeline(
                app, task_id, Some(&image_base64), Some(text), "completed", None,
            );
        }
        Err(e) => {
            eprintln!("[TaskManager] Extraction failed: {}", e);
            finish_extract_pipeline(
                app, task_id, Some(&image_base64), None, "failed", Some(e),
            );
        }
    }
}

fn finish_extract_pipeline(
    app: &AppHandle,
    task_id: u32,
    screenshot_base64: Option<&str>,
    extract_text: Option<String>,
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

    // Update bubble
    let bubble_delay = if status == "completed" { 1500 } else { 3000 };
    let _ = crate::bubble::update(app, task_id, status);
    let _ = crate::bubble::hide(app, task_id, bubble_delay);

    // Save history
    if let Err(e) = mgr.history.add_extract_record(
        screenshot_base64,
        extract_text,
        status,
        error_message,
    ) {
        eprintln!("[TaskManager] Failed to add extract history record: {}", e);
    }

    // Emit updated records
    let records = mgr.history.get_records();
    let json_records = serde_json::to_value(records).unwrap_or(serde_json::json!([]));
    let _ = app.emit("history-updated", json_records);

    // Decrement active count
    mgr.active_count = mgr.active_count.saturating_sub(1);
}

/// Platform-specific screenshot capture. Returns base64-encoded PNG or None if cancelled/failed.
async fn capture_screenshot(app: &AppHandle, task_id: u32) -> Option<String> {
    #[cfg(target_os = "macos")]
    {
        return capture_screenshot_macos(app, task_id).await;
    }

    #[cfg(target_os = "windows")]
    {
        return capture_screenshot_windows(app, task_id).await;
    }

    #[allow(unreachable_code)]
    {
        eprintln!("[TaskManager] Screenshot not supported on this platform");
        let state = app.state::<SharedTaskManager>();
        let mut mgr = state.lock().unwrap();
        mgr.cancel_tokens.remove(&task_id);
        mgr.active_count = mgr.active_count.saturating_sub(1);
        None
    }
}

#[cfg(target_os = "macos")]
async fn capture_screenshot_macos(app: &AppHandle, task_id: u32) -> Option<String> {
    let tmp_path = std::env::temp_dir().join(format!("byetype_capture_{}.png", task_id));

    let capture_result = tokio::process::Command::new("screencapture")
        .arg("-i")
        .arg(tmp_path.as_os_str())
        .output()
        .await;

    let exited_ok = match &capture_result {
        Ok(output) => {
            if !output.stderr.is_empty() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                eprintln!("[TaskManager] screencapture stderr: {}", stderr.trim());
            }
            output.status.success()
        }
        Err(e) => {
            eprintln!("[TaskManager] screencapture failed to launch: {}", e);
            false
        }
    };

    if !exited_ok || !tmp_path.exists() {
        let _ = std::fs::remove_file(&tmp_path);
        let state = app.state::<SharedTaskManager>();
        let mut mgr = state.lock().unwrap();
        mgr.cancel_tokens.remove(&task_id);
        mgr.active_count = mgr.active_count.saturating_sub(1);
        return None;
    }

    let png_bytes = match std::fs::read(&tmp_path) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("[TaskManager] Failed to read screenshot: {}", e);
            let _ = std::fs::remove_file(&tmp_path);
            finish_extract_pipeline(
                app, task_id, None, None, "failed",
                Some(format!("Failed to read screenshot: {}", e)),
            );
            return None;
        }
    };
    let _ = std::fs::remove_file(&tmp_path);
    Some(base64::engine::general_purpose::STANDARD.encode(&png_bytes))
}

#[cfg(target_os = "windows")]
async fn capture_screenshot_windows(app: &AppHandle, task_id: u32) -> Option<String> {
    // Launch Snipping Tool in clipboard mode
    let capture_result = tokio::process::Command::new("snippingtool")
        .arg("/clip")
        .output()
        .await;

    let exited_ok = match &capture_result {
        Ok(output) => output.status.success(),
        Err(e) => {
            eprintln!("[TaskManager] snippingtool failed to launch: {}, trying ms-screenclip", e);
            // Fallback: try ms-screenclip URI scheme
            let fallback = tokio::process::Command::new("explorer")
                .arg("ms-screenclip:")
                .output()
                .await;
            // ms-screenclip returns immediately; wait a moment for user to complete capture
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
            match &fallback {
                Ok(output) => output.status.success(),
                Err(e2) => {
                    eprintln!("[TaskManager] ms-screenclip also failed: {}", e2);
                    false
                }
            }
        }
    };

    if !exited_ok {
        let state = app.state::<SharedTaskManager>();
        let mut mgr = state.lock().unwrap();
        mgr.cancel_tokens.remove(&task_id);
        mgr.active_count = mgr.active_count.saturating_sub(1);
        return None;
    }

    // Read image from clipboard
    let png_bytes = match clipboard_image_to_png() {
        Ok(bytes) => bytes,
        Err(e) => {
            eprintln!("[TaskManager] Failed to read clipboard image: {}", e);
            // User likely cancelled — silent cleanup
            let state = app.state::<SharedTaskManager>();
            let mut mgr = state.lock().unwrap();
            mgr.cancel_tokens.remove(&task_id);
            mgr.active_count = mgr.active_count.saturating_sub(1);
            return None;
        }
    };

    Some(base64::engine::general_purpose::STANDARD.encode(&png_bytes))
}

#[cfg(target_os = "windows")]
fn clipboard_image_to_png() -> Result<Vec<u8>, String> {
    let mut clipboard = arboard::Clipboard::new()
        .map_err(|e| format!("Failed to access clipboard: {}", e))?;
    let img_data = clipboard.get_image()
        .map_err(|e| format!("No image in clipboard: {}", e))?;

    // Convert RGBA data to PNG bytes using image crate
    use image::{ImageBuffer, RgbaImage};
    let rgba: RgbaImage = ImageBuffer::from_raw(
        img_data.width as u32,
        img_data.height as u32,
        img_data.bytes.into_owned(),
    )
    .ok_or_else(|| "Failed to create image buffer from clipboard data".to_string())?;

    let mut png_buf: Vec<u8> = Vec::new();
    let mut cursor = std::io::Cursor::new(&mut png_buf);
    rgba.write_to(&mut cursor, image::ImageFormat::Png)
        .map_err(|e| format!("Failed to encode PNG: {}", e))?;

    Ok(png_buf)
}

/// Frontend calls this to get the full screenshot for display in the overlay.
#[tauri::command]
pub fn get_screenshot_image(
    state: tauri::State<'_, ScreenshotImageState>,
) -> Option<String> {
    state.lock().unwrap().clone()
}

/// Frontend calls this when user finishes or cancels region selection.
#[tauri::command]
pub fn submit_screenshot_crop(
    crop: Option<ScreenshotCrop>,
    sender_state: tauri::State<'_, ScreenshotSender>,
) {
    if let Some(sender) = sender_state.lock().unwrap().take() {
        let _ = sender.send(crop);
    }
}
