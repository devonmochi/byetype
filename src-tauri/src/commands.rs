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
    let old_config = config_manager.get();
    let old_shortcut = old_config.general.shortcut.clone();
    let old_shortcut2 = old_config.general.shortcut2.clone();
    let old_extract_shortcut = old_config.general.extract_shortcut.clone();
    let old_extract_shortcut2 = old_config.general.extract_shortcut2.clone();
    let old_shortcut_template = old_config.general.shortcut_template.clone();
    let old_shortcut2_template = old_config.general.shortcut2_template.clone();
    let old_extract_shortcut_template = old_config.general.extract_shortcut_template.clone();
    let old_extract_shortcut2_template = old_config.general.extract_shortcut2_template.clone();
    config_manager.update(config.clone())?;

    let shortcuts_changed = config.general.shortcut != old_shortcut
        || config.general.shortcut2 != old_shortcut2
        || config.general.extract_shortcut != old_extract_shortcut
        || config.general.extract_shortcut2 != old_extract_shortcut2
        || config.general.shortcut_template != old_shortcut_template
        || config.general.shortcut2_template != old_shortcut2_template
        || config.general.extract_shortcut_template != old_extract_shortcut_template
        || config.general.extract_shortcut2_template != old_extract_shortcut2_template;
    if shortcuts_changed {
        crate::shortcut::register(&app, (*recorder).clone())?;
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
pub fn update_clipboard_text(text: String) -> Result<(), String> {
    use arboard::Clipboard;
    let mut clipboard = Clipboard::new().map_err(|e| format!("Clipboard error: {}", e))?;
    clipboard.set_text(&text).map_err(|e| format!("Set clipboard failed: {}", e))?;
    Ok(())
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

    if ai::is_deepseek(&resolved) {
        let result = ai::deepseek::test_connectivity(
            &client,
            &resolved.api_key,
            &resolved.model,
            &resolved.base_url,
        )
        .await;
        let latency = start.elapsed().as_millis() as u64;
        return match result {
            Ok(()) => Ok(ConnectivityResult { success: true, latency_ms: latency, error: None }),
            Err(e) => Ok(ConnectivityResult { success: false, latency_ms: latency, error: Some(e) }),
        };
    }

    let result = match resolved.protocol.as_str() {
        "gemini" => {
            ai::gemini::test_connectivity(&client, &resolved.api_key, &resolved.model, &resolved.base_url).await
        }
        "qwen-omni" => {
            ai::openai_compat::qwen_omni_test_connectivity(&client, &resolved.api_key, &resolved.model, &resolved.base_url).await
        }
        "mimo" => {
            ai::mimo::test_connectivity(&client, &resolved.api_key, &resolved.model, &resolved.base_url).await
        }
        "longcat" => {
            ai::longcat::test_connectivity(&client, &resolved.api_key, &resolved.model, &resolved.base_url).await
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

// ==================== Backup ====================

use crate::backup::s3 as s3_backup;
use crate::backup::local as local_backup;
use crate::backup::archive::create_backup_zip;
use tauri_plugin_dialog::DialogExt;

#[tauri::command]
pub async fn test_s3_connection(config_manager: State<'_, ConfigManager>) -> Result<(), String> {
    let config = config_manager.get();
    let s3_config = &config.backup.s3;
    s3_backup::test_connection(s3_config).await
}

#[tauri::command]
pub async fn backup_to_s3(
    app: tauri::AppHandle,
    config_manager: State<'_, ConfigManager>,
) -> Result<String, String> {
    let config = config_manager.get();
    let s3_config = &config.backup.s3;

    if s3_config.bucket.is_empty() {
        return Err("请先在设置中配置 S3".to_string());
    }

    let data_dir = app.path().app_data_dir()
        .map_err(|e| format!("获取数据目录失败: {}", e))?;

    let zip_data = create_backup_zip(&data_dir)?;
    let key = s3_backup::generate_backup_key(&s3_config.prefix);

    s3_backup::upload(s3_config, zip_data, key).await
}

#[tauri::command]
pub async fn list_s3_backups(
    config_manager: State<'_, ConfigManager>,
) -> Result<Vec<s3_backup::BackupEntry>, String> {
    let config = config_manager.get();
    let s3_config = &config.backup.s3;

    if s3_config.bucket.is_empty() {
        return Err("请先在设置中配置 S3".to_string());
    }

    s3_backup::list_backups(s3_config).await
}

#[tauri::command]
pub async fn restore_from_s3(
    app: tauri::AppHandle,
    config_manager: State<'_, ConfigManager>,
    object_key: String,
) -> Result<(), String> {
    let config = config_manager.get();
    let s3_config = &config.backup.s3;

    let zip_data = s3_backup::download(s3_config, &object_key).await?;

    let data_dir = app.path().app_data_dir()
        .map_err(|e| format!("获取数据目录失败: {}", e))?;

    // 使用 local_backup 的恢复逻辑（它处理回滚）
    // 先写入临时文件，再调用 restore_from_local
    let temp_zip = data_dir.join(".s3_restore_temp.zip");
    std::fs::write(&temp_zip, &zip_data)
        .map_err(|e| format!("写入临时文件失败: {}", e))?;

    let result = local_backup::restore_from_local(&temp_zip, &data_dir);

    // 清理临时文件
    let _ = std::fs::remove_file(&temp_zip);

    result
}

#[tauri::command]
pub async fn backup_to_local(
    app: tauri::AppHandle,
) -> Result<String, String> {
    let data_dir = app.path().app_data_dir()
        .map_err(|e| format!("获取数据目录失败: {}", e))?;

    let now = chrono::Local::now();
    let timestamp = now.format("%Y%m%d-%H%M%S").to_string();
    let default_name = format!("byetype-backup-{}.zip", timestamp);

    // 弹出文件保存对话框
    let (sender, receiver) = std::sync::mpsc::channel();
    app.dialog()
        .file()
        .add_filter("ZIP 文件", &["zip"])
        .set_file_name(&default_name)
        .save_file(move |file_path| {
            let _ = sender.send(file_path);
        });

    let file_path = receiver.recv()
        .map_err(|_| "文件对话框取消或超时".to_string())?;

    let path = match file_path {
        Some(p) => p,
        None => return Err("未选择保存位置".to_string()),
    };

    let save_path = std::path::PathBuf::from(path.to_string());
    local_backup::backup_to_local(&data_dir, &save_path)
}

#[tauri::command]
pub async fn restore_from_local(
    app: tauri::AppHandle,
) -> Result<(), String> {
    let data_dir = app.path().app_data_dir()
        .map_err(|e| format!("获取数据目录失败: {}", e))?;

    // 弹出文件选择对话框
    let (sender, receiver) = std::sync::mpsc::channel();
    app.dialog()
        .file()
        .add_filter("ZIP 文件", &["zip"])
        .pick_file(move |file_path| {
            let _ = sender.send(file_path);
        });

    let file_path = receiver.recv()
        .map_err(|_| "文件对话框取消或超时".to_string())?;

    let path = match file_path {
        Some(p) => p,
        None => return Err("未选择备份文件".to_string()),
    };

    let zip_path = std::path::PathBuf::from(path.to_string());
    local_backup::restore_from_local(&zip_path, &data_dir)
}

