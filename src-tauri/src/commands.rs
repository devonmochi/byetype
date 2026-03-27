use std::path::PathBuf;
use std::sync::Arc;
use cpal::traits::{DeviceTrait, HostTrait};
use serde::{Deserialize, Serialize};
use tauri::{Manager, State};
use crate::audio::recorder::AudioRecorder;
use crate::config::ConfigManager;
use crate::config::types::AppConfig;
use crate::ai;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioDevice {
    pub name: String,
    pub is_default: bool,
}

/// Public wrapper so lib.rs can call resolve_prompts_dir at setup time.
pub fn resolve_prompts_dir_pub(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    resolve_prompts_dir(app)
}

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
pub fn open_file(path: String) -> Result<(), String> {
    open::that(&path).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_recording_state(recorder: State<'_, Arc<AudioRecorder>>) -> Result<bool, String> {
    Ok(recorder.is_recording())
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
pub fn get_history(
    state: State<'_, crate::task::SharedTaskManager>,
) -> Result<serde_json::Value, String> {
    let mgr = state.lock().unwrap();
    serde_json::to_value(mgr.get_records())
        .map_err(|e| format!("Failed to serialize history: {}", e))
}

#[tauri::command]
pub fn retry_record(
    app: tauri::AppHandle,
    record_id: u64,
) -> Result<(), String> {
    crate::task::retry_record(&app, record_id);
    Ok(())
}

#[tauri::command]
pub fn cancel_task(
    app: tauri::AppHandle,
    task_id: u32,
) -> Result<(), String> {
    crate::task::cancel_task(&app, task_id);
    Ok(())
}

#[tauri::command]
pub fn list_input_devices() -> Result<Vec<AudioDevice>, String> {
    let host = cpal::default_host();
    let default_name = host.default_input_device()
        .and_then(|d| d.name().ok())
        .unwrap_or_default();

    let mut devices = vec![AudioDevice {
        name: "system-default".to_string(),
        is_default: false,
    }];

    if let Ok(input_devices) = host.input_devices() {
        for device in input_devices {
            if let Ok(name) = device.name() {
                devices.push(AudioDevice {
                    name: name.clone(),
                    is_default: name == default_name,
                });
            }
        }
    }

    Ok(devices)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectivityResult {
    pub success: bool,
    pub latency_ms: u64,
    pub error: Option<String>,
}

#[tauri::command]
pub async fn test_model_connectivity(
    config_manager: State<'_, ConfigManager>,
    model_id: String,
) -> Result<ConnectivityResult, String> {
    let config = config_manager.get();
    let resolved = ai::models::resolve_model(&config, &model_id)?;

    if resolved.api_key.is_empty() {
        return Ok(ConnectivityResult {
            success: false,
            latency_ms: 0,
            error: Some("请先填写 API Key".to_string()),
        });
    }

    let client = reqwest::Client::new();
    let start = std::time::Instant::now();

    let result = match resolved.protocol.as_str() {
        "gemini" => {
            ai::gemini::test_connectivity(&client, &resolved.api_key, &resolved.model, &resolved.base_url).await
        }
        _ => {
            ai::openai_compat::test_connectivity(&client, &resolved.api_key, &resolved.model, &resolved.base_url).await
        }
    };

    let latency = start.elapsed().as_millis() as u64;

    match result {
        Ok(()) => Ok(ConnectivityResult { success: true, latency_ms: latency, error: None }),
        Err(e) => Ok(ConnectivityResult { success: false, latency_ms: latency, error: Some(e) }),
    }
}

