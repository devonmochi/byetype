mod ai;
mod task;
mod audio;
mod bubble;
mod clipboard;
mod config;
mod commands;
mod shortcut;
mod tray;
mod updater;

use std::sync::{Arc, Mutex};
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
        .plugin(tauri_plugin_autostart::init(tauri_plugin_autostart::MacosLauncher::LaunchAgent, None))
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(config_manager)
        .manage(recorder.clone())
        .invoke_handler(tauri::generate_handler![
            commands::get_config,
            commands::save_config,
            commands::get_prompts_dir,
            commands::get_builtin_prompt_path,
            commands::copy_builtin_prompt,
            commands::is_builtin_prompt_path,
            commands::open_file,
            commands::get_recording_state,
            commands::set_launch_at_login,
            commands::get_launch_at_login,
            commands::check_for_updates,
            commands::get_history,
            commands::retry_record,
            commands::cancel_task,
        ])
        .setup(move |app| {
            let app_handle = app.handle().clone();

            // Keep app as menu bar accessory (no Dock icon)
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            tray::create(&app_handle)
                .expect("Failed to create system tray");

            // Initialize TaskManager
            let data_dir = app.path().app_data_dir()
                .expect("Failed to resolve app_data_dir");
            let prompts_dir = commands::resolve_prompts_dir_pub(&app_handle)
                .expect("Failed to resolve prompts_dir");
            let task_manager: task::SharedTaskManager =
                Arc::new(Mutex::new(task::TaskManager::new(&data_dir, prompts_dir)));
            app.manage(task_manager);

            let shortcut_key = if initial_shortcut.is_empty() {
                "F4".to_string()
            } else {
                initial_shortcut
            };
            bubble::init(&app_handle)
                .expect("Failed to pre-create bubble window");

            shortcut::register(&app_handle, &shortcut_key, recorder.clone())
                .expect("Failed to register shortcut");

            // Settings window: hidden on startup
            if let Some(win) = app.get_webview_window("settings") {
                let _ = win.hide();
            }

            // Auto-check for updates after a short delay
            let update_handle = app_handle.clone();
            tauri::async_runtime::spawn(async move {
                tokio::time::sleep(std::time::Duration::from_secs(3)).await;
                updater::check_and_prompt_update(update_handle, true).await;
            });

            Ok(())
        })
        .on_window_event(|window, event| {
            // Intercept settings window close: hide instead of destroy, revert to Accessory
            if window.label() == "settings" {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    let _ = window.hide();
                    #[cfg(target_os = "macos")]
                    let _ = window.app_handle().set_activation_policy(tauri::ActivationPolicy::Accessory);
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
