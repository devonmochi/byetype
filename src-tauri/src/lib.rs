mod ai;
mod task;
mod audio;
mod bubble;
mod clipboard;
mod config;
mod commands;
mod preview;
mod shortcut;
mod tray;
mod updater;
mod debug_log;

use std::sync::{Arc, Mutex};
use tauri::Manager;
use config::ConfigManager;
use task::{ScreenshotSender, ScreenshotImageState};
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
            updater::check_update,
            updater::download_update,
            updater::install_and_restart,
            commands::get_history,
            commands::retry_record,
            commands::cancel_task,
            commands::list_input_devices,
            commands::test_model_connectivity,
            commands::update_clipboard_text,
            task::get_screenshot_image,
            task::submit_screenshot_crop,
            preview::set_preview_pinned,
            preview::close_preview_window,
            debug_log::js_debug_log,
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
            debug_log::init(&data_dir);
            let prompts_dir = commands::resolve_prompts_dir_pub(&app_handle)
                .expect("Failed to resolve prompts_dir");
            let task_manager: task::SharedTaskManager =
                Arc::new(Mutex::new(task::TaskManager::new(&data_dir, prompts_dir)));
            app.manage(task_manager);
            app.manage::<ScreenshotSender>(Arc::new(Mutex::new(None)));
            app.manage::<ScreenshotImageState>(Arc::new(Mutex::new(None)));
            app.manage(updater::UpdateState::new(None));

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
                updater::silent_check(update_handle).await;
            });

            Ok(())
        })
        .on_window_event(|window, event| {
            // Intercept settings window close: hide instead of destroy
            if window.label() == "settings" {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    let _ = window.hide();
                    #[cfg(target_os = "macos")]
                    let _ = window.app_handle().set_activation_policy(tauri::ActivationPolicy::Accessory);
                    #[cfg(target_os = "windows")]
                    let _ = window.set_skip_taskbar(true);
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
