use std::path::PathBuf;
use tauri::{Manager, State};
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
    config_manager: State<'_, ConfigManager>,
    config: AppConfig,
) -> Result<bool, String> {
    config_manager.update(config)?;
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
