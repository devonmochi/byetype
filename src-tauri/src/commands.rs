use std::path::PathBuf;
use std::sync::Arc;
use tauri::{Manager, State};
use crate::audio::recorder::AudioRecorder;
use crate::config::ConfigManager;
use crate::config::types::AppConfig;

/// Resolve the builtin prompts directory.
/// In production: resource_dir/prompts
/// In dev: falls back to src-tauri/prompts (next to Cargo.toml)
fn resolve_prompts_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let resource_dir = app.path().resource_dir()
        .map_err(|e| e.to_string())?;
    let prompts_dir = resource_dir.join("prompts");
    if prompts_dir.exists() {
        return Ok(prompts_dir);
    }

    // Dev mode fallback: src-tauri/prompts relative to the manifest dir
    let dev_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("prompts");
    if dev_dir.exists() {
        return Ok(dev_dir);
    }

    Ok(prompts_dir)
}

#[tauri::command]
pub fn get_config(config_manager: State<'_, ConfigManager>) -> Result<AppConfig, String> {
    Ok(config_manager.get())
}

#[tauri::command]
pub fn save_config(
    app: tauri::AppHandle,
    config_manager: State<'_, ConfigManager>,
    recorder: State<'_, Arc<AudioRecorder>>,
    config: AppConfig,
) -> Result<bool, String> {
    let old_shortcut = config_manager.get().general.shortcut.clone();
    config_manager.update(config.clone())?;

    if config.general.shortcut != old_shortcut {
        let new_key = if config.general.shortcut.is_empty() {
            "F4".to_string()
        } else {
            config.general.shortcut.clone()
        };
        crate::shortcut::register(&app, &new_key, (*recorder).clone())?;
    }

    Ok(true)
}

#[tauri::command]
pub fn get_prompts_dir(app: tauri::AppHandle) -> Result<String, String> {
    let prompts_dir = resolve_prompts_dir(&app)?;
    Ok(prompts_dir.to_string_lossy().to_string())
}

#[tauri::command]
pub fn get_builtin_prompt_path(
    app: tauri::AppHandle,
    filename: String,
) -> Result<String, String> {
    let prompts_dir = resolve_prompts_dir(&app)?;
    let path = prompts_dir.join(filename);
    Ok(path.to_string_lossy().to_string())
}

#[tauri::command]
pub fn copy_builtin_prompt(
    app: tauri::AppHandle,
    filename: String,
    force: bool,
) -> Result<String, String> {
    let prompts_dir = resolve_prompts_dir(&app)?;
    let src_path = prompts_dir.join(&filename);

    let data_dir = app.path().app_data_dir()
        .map_err(|e| e.to_string())?;
    let dest_dir = data_dir.join("prompts");
    std::fs::create_dir_all(&dest_dir).map_err(|e| e.to_string())?;
    let dest_path = dest_dir.join(&filename);

    if !force && dest_path.exists() {
        return Ok(dest_path.to_string_lossy().to_string());
    }

    std::fs::copy(&src_path, &dest_path).map_err(|e| e.to_string())?;
    Ok(dest_path.to_string_lossy().to_string())
}

#[tauri::command]
pub fn is_builtin_prompt_path(
    app: tauri::AppHandle,
    path: String,
) -> Result<bool, String> {
    let prompts_dir = resolve_prompts_dir(&app)?;
    Ok(path.starts_with(&prompts_dir.to_string_lossy().as_ref()))
}

#[tauri::command]
pub fn get_theme() -> Result<String, String> {
    Ok("system".to_string())
}

#[tauri::command]
pub fn open_file(path: String) -> Result<(), String> {
    open::that(&path).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_recording_state(recorder: State<'_, Arc<AudioRecorder>>) -> Result<bool, String> {
    Ok(recorder.is_recording())
}

#[tauri::command]
pub fn paste_text(text: String) -> Result<(), String> {
    crate::clipboard::paste_text(&text)
}

#[tauri::command]
pub fn show_bubble(app: tauri::AppHandle, task_id: u32) -> Result<(), String> {
    crate::bubble::show(&app, task_id)
}

#[tauri::command]
pub fn update_bubble(app: tauri::AppHandle, task_id: u32, status: String) -> Result<(), String> {
    crate::bubble::update(&app, task_id, &status)
}

#[tauri::command]
pub fn hide_bubble(app: tauri::AppHandle, task_id: u32, delay_ms: u64) -> Result<(), String> {
    crate::bubble::hide(&app, task_id, delay_ms)
}

#[tauri::command]
pub fn set_launch_at_login(app: tauri::AppHandle, enabled: bool) -> Result<(), String> {
    use tauri_plugin_autostart::ManagerExt;
    let autostart = app.autolaunch();
    if enabled {
        autostart.enable().map_err(|e| e.to_string())
    } else {
        autostart.disable().map_err(|e| e.to_string())
    }
}

#[tauri::command]
pub fn get_launch_at_login(app: tauri::AppHandle) -> Result<bool, String> {
    use tauri_plugin_autostart::ManagerExt;
    app.autolaunch().is_enabled().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn check_for_updates(app: tauri::AppHandle) -> Result<String, String> {
    use tauri_plugin_updater::UpdaterExt;
    match app.updater().map_err(|e| e.to_string())?.check().await {
        Ok(Some(update)) => Ok(format!("v{}", update.version)),
        Ok(None) => Ok(String::new()),
        Err(e) => Err(e.to_string()),
    }
}

#[tauri::command]
pub async fn proxy_request(
    config_manager: State<'_, ConfigManager>,
    request: crate::proxy::ProxyRequest,
) -> Result<crate::proxy::ProxyResponse, String> {
    let proxy_url = config_manager.get().advanced.proxy_url.clone();
    crate::proxy::forward_request(&proxy_url, request).await
}

#[tauri::command]
pub fn get_history(app: tauri::AppHandle) -> Result<serde_json::Value, String> {
    let data_dir = app.path().app_data_dir()
        .map_err(|e| e.to_string())?;
    let history_path = data_dir.join("history").join("history.json");
    if !history_path.exists() {
        return Ok(serde_json::json!([]));
    }
    let content = std::fs::read_to_string(&history_path)
        .map_err(|e| format!("Failed to read history: {}", e))?;
    let records: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse history: {}", e))?;
    Ok(records)
}

#[tauri::command]
pub fn retry_record(app: tauri::AppHandle, record_id: u64) -> Result<(), String> {
    use tauri::Emitter;
    app.emit("retry-request", serde_json::json!({ "recordId": record_id }))
        .map_err(|e| format!("Failed to emit retry request: {}", e))
}
