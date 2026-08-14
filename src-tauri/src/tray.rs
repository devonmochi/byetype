use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager,
};
use tauri_plugin_dialog::{DialogExt, MessageDialogKind};

/// Show the settings window by moving it back on-screen and focusing it.
fn show_settings(app: &AppHandle) {
    if let Some(win) = app.get_webview_window("settings") {
        // Temporarily become a Regular app so macOS activates the window
        #[cfg(target_os = "macos")]
        let _ = app.set_activation_policy(tauri::ActivationPolicy::Regular);
        #[cfg(target_os = "windows")]
        let _ = win.set_skip_taskbar(false);

        let _ = win.center();
        let _ = win.show();
        let _ = win.set_focus();
    }
}

pub fn create(app: &AppHandle) -> Result<(), String> {
    let settings_item = MenuItem::with_id(app, "settings", "设置", true, None::<&str>)
        .map_err(|e| e.to_string())?;
    let history_item = MenuItem::with_id(app, "history", "历史记录", true, None::<&str>)
        .map_err(|e| e.to_string())?;
    let learning_item = MenuItem::with_id(app, "auto_learning", "自动学习", true, None::<&str>)
        .map_err(|e| e.to_string())?;
    let about_item =
        MenuItem::with_id(app, "about", "关于", true, None::<&str>).map_err(|e| e.to_string())?;
    let quit_item =
        MenuItem::with_id(app, "quit", "退出", true, None::<&str>).map_err(|e| e.to_string())?;

    let menu = Menu::with_items(
        app,
        &[
            &settings_item,
            &history_item,
            &learning_item,
            &about_item,
            &quit_item,
        ],
    )
    .map_err(|e| e.to_string())?;

    let icon_bytes = include_bytes!("../icons/tray-default.png");
    let icon = tauri::image::Image::from_bytes(icon_bytes)
        .map_err(|e| format!("Failed to load tray icon: {}", e))?;

    let learning_status_item = learning_item.clone();
    TrayIconBuilder::with_id("main-tray")
        .icon(icon)
        .tooltip("ByeType")
        .menu(&menu)
        .on_menu_event(move |app, event| match event.id().as_ref() {
            "settings" => {
                show_settings(app);
                let _ = app.emit(
                    "navigate-to-tab",
                    crate::updater::NavigatePayload {
                        tab: "general".to_string(),
                    },
                );
            }
            "history" => {
                show_settings(app);
                let _ = app.emit(
                    "navigate-to-tab",
                    crate::updater::NavigatePayload {
                        tab: "history".to_string(),
                    },
                );
            }
            "auto_learning" => {
                let app_handle = app.clone();
                let status_item = learning_status_item.clone();
                let _ = status_item.set_enabled(false);
                let _ = status_item.set_text("自动学习中…");
                tauri::async_runtime::spawn(async move {
                    let result = crate::learning::learn_from_clipboard(&app_handle).await;
                    let _ = status_item.set_text("自动学习");
                    let _ = status_item.set_enabled(true);

                    if let Err(error) = result {
                        app_handle
                            .dialog()
                            .message(error)
                            .title("自动学习")
                            .kind(MessageDialogKind::Error)
                            .show(|_| {});
                    }
                });
            }
            "about" => {
                show_settings(app);
                let _ = app.emit(
                    "navigate-to-tab",
                    crate::updater::NavigatePayload {
                        tab: "about".to_string(),
                    },
                );
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_settings(tray.app_handle());
            }
        })
        .build(app)
        .map_err(|e| format!("Failed to create tray: {}", e))?;

    Ok(())
}
