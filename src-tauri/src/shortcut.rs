use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

use crate::audio::recorder::AudioRecorder;
use crate::FrontAppState;

/// Get the frontmost application name on macOS via osascript.
fn get_frontmost_app() -> Option<String> {
    #[cfg(target_os = "macos")]
    {
        let output = std::process::Command::new("osascript")
            .arg("-e")
            .arg("tell application \"System Events\" to get name of first application process whose frontmost is true")
            .output()
            .ok()?;
        let name = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if name.is_empty() { None } else { Some(name) }
    }
    #[cfg(not(target_os = "macos"))]
    { None }
}

/// Register the global shortcut that toggles recording.
pub fn register(
    app: &AppHandle,
    shortcut_key: &str,
    recorder: Arc<AudioRecorder>,
) -> Result<(), String> {
    let app_handle = app.clone();

    app.global_shortcut().unregister_all()
        .map_err(|e| format!("Failed to unregister shortcuts: {}", e))?;

    app.global_shortcut()
        .on_shortcut(shortcut_key, move |_app, _shortcut, event| {
            if event.state != ShortcutState::Pressed {
                return;
            }

            if recorder.is_recording() {
                match recorder.stop() {
                    Ok(base64_audio) => {
                        update_tray_icon(&app_handle, false);
                        let _ = app_handle.emit("recording-complete", serde_json::json!({
                            "audio": base64_audio
                        }));
                    }
                    Err(e) => {
                        eprintln!("Stop recording error: {}", e);
                        let _ = app_handle.emit("recording-error", serde_json::json!({
                            "message": e
                        }));
                    }
                }
            } else {
                // Save the frontmost app before recording starts
                if let Some(front_app_state) = app_handle.try_state::<FrontAppState>() {
                    let front_app = get_frontmost_app();
                    if let Ok(mut state) = front_app_state.0.lock() {
                        *state = front_app;
                    }
                }
                match recorder.start() {
                    Ok(()) => {
                        update_tray_icon(&app_handle, true);
                        let _ = app_handle.emit("recording-started", ());
                    }
                    Err(e) => {
                        eprintln!("Start recording error: {}", e);
                        let _ = app_handle.emit("recording-error", serde_json::json!({
                            "message": e
                        }));
                    }
                }
            }
        })
        .map_err(|e| format!("Failed to register shortcut '{}': {}", shortcut_key, e))?;

    Ok(())
}

fn update_tray_icon(app: &AppHandle, is_recording: bool) {
    if let Some(tray) = app.tray_by_id("main-tray") {
        let icon_bytes: &[u8] = if is_recording {
            include_bytes!("../icons/tray-recording.png")
        } else {
            include_bytes!("../icons/tray-default.png")
        };
        if let Ok(icon) = tauri::image::Image::from_bytes(icon_bytes) {
            let _ = tray.set_icon(Some(icon));
        }
    }
}
