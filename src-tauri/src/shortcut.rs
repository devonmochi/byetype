use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicU32, Ordering};
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

use crate::audio::recorder::AudioRecorder;
use crate::config::ConfigManager;

/// Register all 4 global shortcuts (2 voice + 2 image), each bound to its own template_id.
pub fn register(
    app: &AppHandle,
    recorder: Arc<AudioRecorder>,
) -> Result<(), String> {
    let cfg = app.state::<ConfigManager>().get();
    let keys = [
        cfg.general.shortcut.clone(),
        cfg.general.shortcut2.clone(),
        cfg.general.extract_shortcut.clone(),
        cfg.general.extract_shortcut2.clone(),
    ];

    // Conflict detection: all 4 shortcuts must be unique
    for i in 0..keys.len() {
        for j in (i + 1)..keys.len() {
            if !keys[i].is_empty() && keys[i] == keys[j] {
                return Err(format!("快捷键 '{}' 重复，请设置不同的快捷键", keys[i]));
            }
        }
    }

    app.global_shortcut().unregister_all()
        .map_err(|e| format!("Failed to unregister shortcuts: {}", e))?;

    // Voice shortcut 1
    if !cfg.general.shortcut.is_empty() {
        register_voice_shortcut(app, &cfg.general.shortcut, cfg.general.shortcut_template.clone(), recorder.clone())?;
    }
    // Voice shortcut 2
    if !cfg.general.shortcut2.is_empty() {
        register_voice_shortcut(app, &cfg.general.shortcut2, cfg.general.shortcut2_template.clone(), recorder.clone())?;
    }
    // Image shortcut 1
    let ek1 = if cfg.general.extract_shortcut.is_empty() { "F6".to_string() } else { cfg.general.extract_shortcut.clone() };
    register_image_shortcut(app, &ek1, cfg.general.extract_shortcut_template.clone())?;
    // Image shortcut 2
    if !cfg.general.extract_shortcut2.is_empty() {
        register_image_shortcut(app, &cfg.general.extract_shortcut2, cfg.general.extract_shortcut2_template.clone())?;
    }

    Ok(())
}

/// Register a single voice (recording-toggle) shortcut bound to the given template_id.
fn register_voice_shortcut(
    app: &AppHandle,
    shortcut_key: &str,
    template_id: String,
    recorder: Arc<AudioRecorder>,
) -> Result<(), String> {
    let app_handle = app.clone();
    let tmpl = template_id;
    // Track the current recording's task_id between start and stop
    let current_task_id: Arc<Mutex<Option<u32>>> = Arc::new(Mutex::new(None));
    let recording_gen: Arc<AtomicU32> = Arc::new(AtomicU32::new(0));

    app.global_shortcut()
        .on_shortcut(shortcut_key, move |_app, _shortcut, event| {
            if event.state != ShortcutState::Pressed {
                return;
            }

            if recorder.is_recording() {
                // CAS: claim the right to stop — advance gen to invalidate timer
                let gen = recording_gen.load(Ordering::SeqCst);
                if recording_gen.compare_exchange(gen, gen + 1, Ordering::SeqCst, Ordering::SeqCst).is_err() {
                    return; // Timer already stopped it
                }

                let task_id = current_task_id.lock().unwrap().take();
                match recorder.stop() {
                    Ok(base64_audio) => {
                        update_tray_icon(&app_handle, false);
                        if let Some(tid) = task_id {
                            crate::task::process_recording(&app_handle, tid, base64_audio, tmpl.clone());
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
                let mic = app_handle.state::<ConfigManager>().get()
                    .general.microphone.clone();
                match recorder.start(&mic) {
                    Ok(()) => {
                        // Bump generation to mark a new recording session
                        let gen = recording_gen.fetch_add(1, Ordering::SeqCst) + 1;

                        update_tray_icon(&app_handle, true);
                        let tid = crate::task::start_recording(&app_handle);
                        *current_task_id.lock().unwrap() = tid;

                        // Read config for max recording duration
                        let max_secs = app_handle.state::<ConfigManager>().get()
                            .general.max_recording_seconds;

                        if max_secs > 0 {
                            let t_recorder = recorder.clone();
                            let t_app = app_handle.clone();
                            let t_task_id = current_task_id.clone();
                            let t_gen = recording_gen.clone();
                            let t_tmpl = tmpl.clone();
                            std::thread::spawn(move || {
                                std::thread::sleep(std::time::Duration::from_secs(max_secs as u64));
                                // CAS: claim the right to stop — advance gen; fails if manually stopped or new recording started
                                if t_gen.compare_exchange(gen, gen + 1, Ordering::SeqCst, Ordering::SeqCst).is_ok() {
                                    let task_id = t_task_id.lock().unwrap().take();
                                    match t_recorder.stop() {
                                        Ok(base64_audio) => {
                                            update_tray_icon(&t_app, false);
                                            if let Some(tid) = task_id {
                                                crate::task::process_recording(&t_app, tid, base64_audio, t_tmpl.clone());
                                            }
                                        }
                                        Err(e) => {
                                            eprintln!("Auto-stop recording error: {}", e);
                                            if let Some(tid) = task_id {
                                                crate::task::cancel_recording(&t_app, tid);
                                            }
                                            update_tray_icon(&t_app, false);
                                            let _ = t_app.emit("recording-error", serde_json::json!({
                                                "message": e
                                            }));
                                        }
                                    }
                                }
                            });
                        }
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
        .map_err(|e| format!("Failed to register voice shortcut '{}': {}", shortcut_key, e))?;

    Ok(())
}

/// Register a single image (screenshot + OCR extraction) shortcut bound to the given template_id.
fn register_image_shortcut(
    app: &AppHandle,
    shortcut_key: &str,
    template_id: String,
) -> Result<(), String> {
    let extract_app = app.clone();
    let tmpl = template_id;
    app.global_shortcut()
        .on_shortcut(shortcut_key, move |_app, _shortcut, event| {
            if event.state != ShortcutState::Pressed { return; }
            crate::task::start_extraction(&extract_app, tmpl.clone());
        })
        .map_err(|e| format!("Failed to register image shortcut '{}': {}", shortcut_key, e))?;
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
