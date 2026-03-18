use tauri::{AppHandle, Manager, Emitter, WebviewUrl, WebviewWindowBuilder};

const BUBBLE_SIZE: f64 = 40.0;
const OFFSET_X: f64 = 10.0;
const OFFSET_Y: f64 = 10.0;

fn cursor_position() -> (f64, f64) {
    #[cfg(target_os = "macos")]
    {
        use core_graphics::event::CGEvent;
        use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};
        if let Ok(source) = CGEventSource::new(CGEventSourceStateID::HIDSystemState) {
            if let Ok(event) = CGEvent::new(source) {
                let point = event.location();
                return (point.x, point.y);
            }
        }
    }
    (100.0, 100.0)
}

/// Save the frontmost application so we can restore focus after creating a window.
#[cfg(target_os = "macos")]
fn save_frontmost_app() -> Option<*mut objc::runtime::Object> {
    use objc::runtime::{Class, Object};
    use objc::msg_send;
    use objc::sel;
    use objc::sel_impl;
    unsafe {
        let ws_cls = Class::get("NSWorkspace")?;
        let ws: *mut Object = msg_send![ws_cls, sharedWorkspace];
        let front_app: *mut Object = msg_send![ws, frontmostApplication];
        if front_app.is_null() {
            None
        } else {
            // Retain so it stays alive
            let _: *mut Object = msg_send![front_app, retain];
            Some(front_app)
        }
    }
}

/// Reactivate a previously saved application.
#[cfg(target_os = "macos")]
fn restore_frontmost_app(app_obj: *mut objc::runtime::Object) {
    use objc::msg_send;
    use objc::sel;
    use objc::sel_impl;
    unsafe {
        // NSApplicationActivateIgnoringOtherApps = 1 << 1 = 2
        let _: objc::runtime::BOOL = msg_send![app_obj, activateWithOptions: 2usize];
        let _: () = msg_send![app_obj, release];
    }
}

pub fn show(app: &AppHandle, task_id: u32) -> Result<(), String> {
    let label = format!("bubble-{}", task_id);

    if let Some(win) = app.get_webview_window(&label) {
        let _ = win.close();
    }

    // Save current foreground app before creating the window
    #[cfg(target_os = "macos")]
    let prev_app = save_frontmost_app();

    let (cx, cy) = cursor_position();
    let x = cx + OFFSET_X;
    let y = cy + OFFSET_Y;

    // Pass initial state via URL query params so JS can render immediately
    let url = format!("bubble.html?task={}&status=recording", task_id);

    WebviewWindowBuilder::new(
        app,
        &label,
        WebviewUrl::App(url.into()),
    )
    .title("")
    .inner_size(BUBBLE_SIZE, BUBBLE_SIZE)
    .position(x, y)
    .decorations(false)
    .always_on_top(true)
    .skip_taskbar(true)
    .resizable(false)
    .focused(false)
    .visible(true)
    .transparent(true)
    .shadow(false)
    .build()
    .map_err(|e| format!("Failed to create bubble window: {}", e))?;

    // Restore focus to the previously active app
    #[cfg(target_os = "macos")]
    if let Some(prev) = prev_app {
        restore_frontmost_app(prev);
    }

    Ok(())
}

pub fn update(app: &AppHandle, task_id: u32, status: &str) -> Result<(), String> {
    let label = format!("bubble-{}", task_id);
    app.emit_to(
        &label,
        "update-bubble",
        serde_json::json!({ "taskNumber": task_id, "status": status }),
    ).map_err(|e| format!("Failed to update bubble: {}", e))
}

pub fn hide(app: &AppHandle, task_id: u32, delay_ms: u64) -> Result<(), String> {
    let label = format!("bubble-{}", task_id);
    let app_handle = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(delay_ms));
        if let Some(win) = app_handle.get_webview_window(&label) {
            let _ = win.close();
        }
    });
    Ok(())
}
