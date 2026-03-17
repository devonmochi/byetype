mod audio;
mod bubble;
mod clipboard;
mod config;
mod commands;
mod shortcut;
mod tray;

use std::sync::Arc;
use tauri::Manager;
use config::ConfigManager;
use audio::recorder::AudioRecorder;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let config_manager = ConfigManager::new(None);
    let initial_shortcut = config_manager.get().general.shortcut.clone();
    let recorder = Arc::new(AudioRecorder::new());

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_single_instance::init(|_app, _args, _cwd| {}))
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .manage(config_manager)
        .manage(recorder.clone())
        .invoke_handler(tauri::generate_handler![
            commands::get_config,
            commands::save_config,
            commands::get_prompts_dir,
            commands::get_builtin_prompt_path,
            commands::copy_builtin_prompt,
            commands::is_builtin_prompt_path,
            commands::get_theme,
            commands::open_file,
            commands::get_recording_state,
            commands::paste_text,
            commands::show_bubble,
            commands::update_bubble,
            commands::hide_bubble,
        ])
        .setup(move |app| {
            let app_handle = app.handle().clone();

            tray::create(&app_handle)
                .expect("Failed to create system tray");

            let shortcut_key = if initial_shortcut.is_empty() {
                "F4".to_string()
            } else {
                initial_shortcut
            };
            shortcut::register(&app_handle, &shortcut_key, recorder.clone())
                .expect("Failed to register shortcut");

            if let Some(win) = app.get_webview_window("settings") {
                let _ = win.hide();
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
