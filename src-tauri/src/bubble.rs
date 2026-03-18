use tauri::{AppHandle, Manager, Emitter, WebviewUrl, WebviewWindowBuilder};

const BUBBLE_SIZE: f64 = 40.0;
const OFFSET_X: f64 = 10.0;
const OFFSET_Y: f64 = 10.0;
const MAX_BUBBLES: u32 = 3;

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

fn label_for(task_id: u32) -> String {
    format!("bubble-{}", task_id)
}

/// Pre-create a pool of hidden bubble windows at startup.
pub fn init(app: &AppHandle) -> Result<(), String> {
    for i in 1..=MAX_BUBBLES {
        let label = label_for(i);
        WebviewWindowBuilder::new(
            app,
            &label,
            WebviewUrl::App("bubble.html".into()),
        )
        .title("")
        .inner_size(BUBBLE_SIZE, BUBBLE_SIZE)
        .position(-200.0, -200.0)
        .decorations(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .resizable(false)
        .focused(false)
        .visible(false)
        .transparent(true)
        .shadow(false)
        .build()
        .map_err(|e| format!("Failed to pre-create bubble-{}: {}", i, e))?;
    }
    Ok(())
}

pub fn show(app: &AppHandle, task_id: u32) -> Result<(), String> {
    let label = label_for(task_id);

    let (cx, cy) = cursor_position();
    let x = cx + OFFSET_X;
    let y = cy + OFFSET_Y;

    if let Some(win) = app.get_webview_window(&label) {
        let _ = win.set_position(tauri::Position::Logical(
            tauri::LogicalPosition::new(x, y),
        ));
        let _ = app.emit_to(
            &label,
            "show-bubble",
            serde_json::json!({ "taskNumber": task_id, "status": "recording" }),
        );
        let _ = win.show();
    } else {
        eprintln!("[Bubble] Window {} not found in pool", label);
    }

    Ok(())
}

pub fn update(app: &AppHandle, task_id: u32, status: &str) -> Result<(), String> {
    let label = label_for(task_id);
    app.emit_to(
        &label,
        "update-bubble",
        serde_json::json!({ "taskNumber": task_id, "status": status }),
    )
    .map_err(|e| format!("Failed to update bubble: {}", e))
}

pub fn hide(app: &AppHandle, task_id: u32, delay_ms: u64) -> Result<(), String> {
    let label = label_for(task_id);
    let app_handle = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(delay_ms));
        if let Some(win) = app_handle.get_webview_window(&label) {
            let _ = win.hide();
            let _ = win.set_position(tauri::Position::Logical(
                tauri::LogicalPosition::new(-200.0, -200.0),
            ));
        }
    });
    Ok(())
}
