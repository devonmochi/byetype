use tauri::{
    AppHandle, Manager,
    tray::{TrayIconBuilder, MouseButton, MouseButtonState, TrayIconEvent},
    menu::{Menu, MenuItem},
};

/// Show the settings window by moving it back on-screen and focusing it.
fn show_settings(app: &AppHandle) {
    if let Some(win) = app.get_webview_window("settings") {
        // Temporarily become a Regular app so macOS activates the window
        #[cfg(target_os = "macos")]
        let _ = app.set_activation_policy(tauri::ActivationPolicy::Regular);

        let _ = win.center();
        let _ = win.show();
        let _ = win.set_focus();
    }
}

pub fn create(app: &AppHandle) -> Result<(), String> {
    let settings_item = MenuItem::with_id(app, "settings", "设置", true, None::<&str>)
        .map_err(|e| e.to_string())?;
    let check_update_item = MenuItem::with_id(app, "check_update", "检查更新", true, None::<&str>)
        .map_err(|e| e.to_string())?;
    let quit_item = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)
        .map_err(|e| e.to_string())?;

    let menu = Menu::with_items(app, &[&settings_item, &check_update_item, &quit_item])
        .map_err(|e| e.to_string())?;

    let icon_bytes = include_bytes!("../icons/tray-default.png");
    let icon = tauri::image::Image::from_bytes(icon_bytes)
        .map_err(|e| format!("Failed to load tray icon: {}", e))?;

    TrayIconBuilder::with_id("main-tray")
        .icon(icon)
        .tooltip("ByeType")
        .menu(&menu)
        .on_menu_event(|app, event| {
            match event.id().as_ref() {
                "settings" => show_settings(app),
                "check_update" => {
                    let app = app.clone();
                    tauri::async_runtime::spawn(async move {
                        use tauri_plugin_updater::UpdaterExt;
                        match app.updater() {
                            Ok(updater) => match updater.check().await {
                                Ok(Some(update)) => {
                                    println!("Update available: v{}", update.version);
                                }
                                Ok(None) => println!("Already up to date"),
                                Err(e) => eprintln!("Update check failed: {}", e),
                            },
                            Err(e) => eprintln!("Updater init failed: {}", e),
                        }
                    });
                }
                "quit" => app.exit(0),
                _ => {}
            }
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event {
                show_settings(tray.app_handle());
            }
        })
        .build(app)
        .map_err(|e| format!("Failed to create tray: {}", e))?;

    Ok(())
}
