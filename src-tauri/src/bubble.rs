use tauri::{AppHandle, Manager, Emitter, WebviewUrl, WebviewWindowBuilder};
use std::sync::atomic::{AtomicU32, Ordering};

const BUBBLE_WIDTH: f64 = 140.0;
const BUBBLE_HEIGHT: f64 = 64.0;
const OFFSET_X: f64 = 10.0;
const OFFSET_Y: f64 = 10.0;
const MAX_BUBBLES: u32 = 3;

/// Generation counter per bubble slot — prevents stale delayed hides
static SHOW_GEN: [AtomicU32; 3] = [
    AtomicU32::new(0),
    AtomicU32::new(0),
    AtomicU32::new(0),
];

fn gen_index(task_id: u32) -> usize {
    (task_id as usize).saturating_sub(1).min(2)
}

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

    #[cfg(target_os = "windows")]
    {
        use windows_sys::Win32::Foundation::POINT;
        use windows_sys::Win32::UI::WindowsAndMessaging::GetCursorPos;
        let mut point = POINT { x: 0, y: 0 };
        if unsafe { GetCursorPos(&mut point) } != 0 {
            return (point.x as f64, point.y as f64);
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
        let mut builder = WebviewWindowBuilder::new(
            app,
            &label,
            WebviewUrl::App("bubble.html".into()),
        )
        .title("")
        .inner_size(BUBBLE_WIDTH, BUBBLE_HEIGHT)
        .position(-200.0, -200.0)
        .decorations(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .resizable(false)
        .focused(false)
        .visible(false);

        builder = builder.transparent(true).shadow(false);

        builder
            .build()
            .map_err(|e| format!("Failed to pre-create bubble-{}: {}", i, e))?;
    }
    Ok(())
}

pub fn show(app: &AppHandle, task_id: u32) -> Result<(), String> {
    let label = label_for(task_id);

    // Bump generation so any pending hide for this slot is invalidated
    let idx = gen_index(task_id);
    SHOW_GEN[idx].fetch_add(1, Ordering::SeqCst);

    let (cx, cy) = cursor_position();
    let x = cx + OFFSET_X;
    let y = cy + OFFSET_Y;

    if let Some(win) = app.get_webview_window(&label) {
        // Clear old content first to prevent flash of stale state
        let _ = app.emit_to(
            &label,
            "clear-bubble",
            serde_json::json!({}),
        );

        // Windows: GetCursorPos returns physical pixels, use PhysicalPosition
        // macOS: CGEvent.location() returns logical points, use LogicalPosition
        #[cfg(target_os = "windows")]
        let _ = win.set_position(tauri::Position::Physical(
            tauri::PhysicalPosition::new(x as i32, y as i32),
        ));
        #[cfg(not(target_os = "windows"))]
        let _ = win.set_position(tauri::Position::Logical(
            tauri::LogicalPosition::new(x, y),
        ));

        // Show window BEFORE emitting events — on Windows, WebView2 may not
        // process events while the window is hidden.
        let _ = win.show();
        let _ = app.emit_to(
            &label,
            "show-bubble",
            serde_json::json!({ "taskNumber": task_id, "status": "recording" }),
        );
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

    // Capture current generation — if show() bumps it before we wake, skip hide
    let idx = gen_index(task_id);
    let gen_at_schedule = SHOW_GEN[idx].load(Ordering::SeqCst);

    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(delay_ms));

        // Abort if a new show() happened while we were sleeping
        if SHOW_GEN[idx].load(Ordering::SeqCst) != gen_at_schedule {
            return;
        }

        if let Some(win) = app_handle.get_webview_window(&label) {
            // Clear content so next show won't flash stale state
            let _ = app_handle.emit_to(&label, "clear-bubble", serde_json::json!({}));
            let _ = win.hide();
            let _ = win.set_position(tauri::Position::Logical(
                tauri::LogicalPosition::new(-200.0, -200.0),
            ));
        }
    });
    Ok(())
}
