use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

use crate::audio::recorder::AudioRecorder;

/// Register the global shortcut that toggles recording.
pub fn register(
    app: &AppHandle,
    shortcut_key: &str,
    recorder: Arc<AudioRecorder>,
) -> Result<(), String> {
    let app_handle = app.clone();
    // Track the current recording's task_id between start and stop
    let current_task_id: Arc<Mutex<Option<u32>>> = Arc::new(Mutex::new(None));

    app.global_shortcut().unregister_all()
        .map_err(|e| format!("Failed to unregister shortcuts: {}", e))?;

    app.global_shortcut()
        .on_shortcut(shortcut_key, move |_app, _shortcut, event| {
            if event.state != ShortcutState::Pressed {
                return;
            }

            if recorder.is_recording() {
                // Take the task_id that was allocated when recording started
                let task_id = current_task_id.lock().unwrap().take();
                match recorder.stop() {
                    Ok(base64_audio) => {
                        update_tray_icon(&app_handle, false);
                        if let Some(tid) = task_id {
                            crate::task::process_recording(&app_handle, tid, base64_audio);
                        }
                    }
                    Err(e) => {
                        eprintln!("Stop recording error: {}", e);
                        if let Some(tid) = task_id {
                            crate::task::cancel_recording(&app_handle, tid);
                        }
                        update_tray_icon(&app_handle, false);
                        let _ = app_handle.emit("recording-error", serde_json::json!({
                            "message": e
                        }));
                    }
                }
            } else {
                match recorder.start() {
                    Ok(()) => {
                        update_tray_icon(&app_handle, true);
                        // Show bubble immediately when recording starts
                        let tid = crate::task::start_recording(&app_handle);
                        *current_task_id.lock().unwrap() = tid;
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
