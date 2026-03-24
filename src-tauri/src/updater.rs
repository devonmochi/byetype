use std::sync::Mutex;
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_updater::{Update, UpdaterExt};

/// 前端可见的更新信息
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    pub version: String,
    pub body: Option<String>,
}

/// 下载进度事件 payload
#[derive(Clone, Serialize)]
pub struct ProgressPayload {
    pub percent: f64,
}

/// 更新可用事件 payload
#[derive(Clone, Serialize)]
pub struct UpdateAvailablePayload {
    pub version: String,
    pub body: Option<String>,
}

/// 错误事件 payload
#[derive(Clone, Serialize)]
pub struct ErrorPayload {
    pub message: String,
}

/// 导航事件 payload
#[derive(Clone, Serialize)]
pub struct NavigatePayload {
    pub tab: String,
}

/// Tauri 托管状态：缓存 Update 对象和下载的字节数据
/// - check 后存入 (Update, None)
/// - download 后存入 (Update, Some(bytes))
pub type UpdateState = Mutex<Option<(Update, Option<Vec<u8>>)>>;

/// 启动时静默检查更新，有新版本则 emit 事件通知前端
pub async fn silent_check(app: AppHandle) {
    let updater = match app.updater() {
        Ok(u) => u,
        Err(e) => {
            eprintln!("Updater init failed: {}", e);
            return;
        }
    };

    match updater.check().await {
        Ok(Some(update)) => {
            let info = UpdateAvailablePayload {
                version: update.version.clone(),
                body: update.body.clone(),
            };
            if let Some(state) = app.try_state::<UpdateState>() {
                *state.lock().unwrap() = Some((update, None));
            }
            let _ = app.emit("update-available", info);
        }
        Ok(None) => {}
        Err(e) => {
            eprintln!("Silent update check failed: {}", e);
        }
    }
}

/// Tauri 命令：检查更新
#[tauri::command]
pub async fn check_update(app: AppHandle) -> Result<Option<UpdateInfo>, String> {
    let updater = app.updater().map_err(|e| e.to_string())?;

    match updater.check().await {
        Ok(Some(update)) => {
            let info = UpdateInfo {
                version: update.version.clone(),
                body: update.body.clone(),
            };
            let state = app.state::<UpdateState>();
            *state.lock().unwrap() = Some((update, None));
            Ok(Some(info))
        }
        Ok(None) => Ok(None),
        Err(e) => Err(e.to_string()),
    }
}

/// Tauri 命令：下载更新（通过事件推送进度）
#[tauri::command]
pub async fn download_update(app: AppHandle) -> Result<(), String> {
    let update = {
        let state = app.state::<UpdateState>();
        let taken = state.lock().unwrap().take();
        taken.map(|(u, _)| u)
    };

    let update = match update {
        Some(u) => u,
        None => return Err("没有可用的更新，请先检查更新".to_string()),
    };

    let app_clone = app.clone();
    let mut downloaded: f64 = 0.0;

    let bytes = update
        .download(
            move |chunk_length, content_length| {
                downloaded += chunk_length as f64;
                let percent = match content_length {
                    Some(total) if total > 0 => (downloaded / total as f64 * 100.0).min(100.0),
                    _ => 0.0,
                };
                let _ = app_clone.emit("update-progress", ProgressPayload { percent });
            },
            || {},
        )
        .await
        .map_err(|e| {
            let msg = format!("下载失败：{}", e);
            let _ = app.emit("update-error", ErrorPayload { message: msg.clone() });
            msg
        })?;

    let state = app.state::<UpdateState>();
    *state.lock().unwrap() = Some((update, Some(bytes)));

    let _ = app.emit("update-complete", serde_json::json!({}));
    Ok(())
}

/// Tauri 命令：安装更新并重启
#[tauri::command]
pub async fn install_and_restart(app: AppHandle) -> Result<(), String> {
    let (update, bytes) = {
        let state = app.state::<UpdateState>();
        let taken = state.lock().unwrap().take();
        taken.ok_or_else(|| "没有已下载的更新".to_string())?
    };

    let bytes = bytes.ok_or_else(|| "更新尚未下载完成".to_string())?;

    update.install(bytes).map_err(|e| format!("安装失败：{}", e))?;
    app.restart();
}
