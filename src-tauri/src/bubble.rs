use tauri::{AppHandle, Manager, Emitter, WebviewUrl, WebviewWindowBuilder, WebviewWindow};
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

    if let Some(win) = app.get_webview_window(&label) {
        // Clear old content first to prevent flash of stale state
        let _ = app.emit_to(
            &label,
            "clear-bubble",
            serde_json::json!({}),
        );

        // 光标右下偏移,超出屏幕则反向贴边
        position_near_cursor(&win, cx, cy);

        // Show window BEFORE emitting events — on Windows, WebView2 may not
        // process events while the window is hidden.
        // Windows: use SW_SHOWNOACTIVATE to avoid stealing focus from the
        // foreground application (e.g. chat input boxes).
        #[cfg(target_os = "windows")]
        {
            use windows_sys::Win32::UI::WindowsAndMessaging::{ShowWindow, SW_SHOWNOACTIVATE};
            let hwnd = win.hwnd().unwrap().0 as *mut std::ffi::c_void;
            unsafe { ShowWindow(hwnd, SW_SHOWNOACTIVATE); }
        }
        #[cfg(not(target_os = "windows"))]
        {
            let _ = win.show();
        }
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

/// 把 bubble 定位到光标右下偏移处,超出屏幕则反向贴边。
///
/// 坐标系约定:
/// - macOS: 光标 (CGEvent) 与 Monitor API 都用逻辑点 → LogicalPosition。
/// - Windows: 光标 (GetCursorPos) 与 Monitor API 都用物理像素 → PhysicalPosition。
fn position_near_cursor(win: &WebviewWindow, cx: f64, cy: f64) {
    let monitors = match win.available_monitors() {
        Ok(m) if !m.is_empty() => m,
        _ => {
            // 兜底:沿用旧的纯偏移逻辑
            #[cfg(target_os = "windows")]
            { let _ = win.set_position(tauri::Position::Physical(
                tauri::PhysicalPosition::new((cx + OFFSET_X) as i32, (cy + OFFSET_Y) as i32))); }
            #[cfg(not(target_os = "windows"))]
            { let _ = win.set_position(tauri::Position::Logical(
                tauri::LogicalPosition::new(cx + OFFSET_X, cy + OFFSET_Y))); }
            return;
        }
    };

    #[cfg(target_os = "macos")]
    {
        let monitor = monitors.iter().find(|m| {
            let s = m.scale_factor();
            let l = m.position().x as f64 / s;
            let t = m.position().y as f64 / s;
            let r = l + m.size().width as f64 / s;
            let b = t + m.size().height as f64 / s;
            cx >= l && cx < r && cy >= t && cy < b
        }).unwrap_or(&monitors[0]);
        let s = monitor.scale_factor();
        let l = monitor.position().x as f64 / s;
        let t = monitor.position().y as f64 / s;
        let r = l + monitor.size().width as f64 / s;
        let b = t + monitor.size().height as f64 / s;

        let (x, y) = pick_pos(cx, cy, BUBBLE_WIDTH, BUBBLE_HEIGHT, l, t, r, b, OFFSET_X, OFFSET_Y);
        let _ = win.set_position(tauri::Position::Logical(
            tauri::LogicalPosition::new(x, y),
        ));
    }

    #[cfg(target_os = "windows")]
    {
        let monitor = monitors.iter().find(|m| {
            let pos = m.position();
            let size = m.size();
            cx >= pos.x as f64
                && cx < (pos.x + size.width as i32) as f64
                && cy >= pos.y as f64
                && cy < (pos.y + size.height as i32) as f64
        }).unwrap_or(&monitors[0]);
        let s = monitor.scale_factor();
        // bubble 尺寸是逻辑像素,Windows 比较时需转物理
        let pw = BUBBLE_WIDTH * s;
        let ph = BUBBLE_HEIGHT * s;
        let l = monitor.position().x as f64;
        let t = monitor.position().y as f64;
        let r = l + monitor.size().width as f64;
        let b = t + monitor.size().height as f64;
        let off_x = OFFSET_X * s;
        let off_y = OFFSET_Y * s;

        let (x, y) = pick_pos(cx, cy, pw, ph, l, t, r, b, off_x, off_y);
        let _ = win.set_position(tauri::Position::Physical(
            tauri::PhysicalPosition::new(x as i32, y as i32),
        ));
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        let _ = win.set_position(tauri::Position::Logical(
            tauri::LogicalPosition::new(cx + OFFSET_X, cy + OFFSET_Y),
        ));
    }
}

/// 选位:光标右下放得下就右下,否则反向(左上),最后 clamp 到屏内兜底。
#[allow(clippy::too_many_arguments)]
fn pick_pos(
    cx: f64, cy: f64, w: f64, h: f64,
    l: f64, t: f64, r: f64, b: f64,
    off_x: f64, off_y: f64,
) -> (f64, f64) {
    let mut x = cx + off_x;
    let mut y = cy + off_y;
    if x + w > r { x = cx - off_x - w; }
    if y + h > b { y = cy - off_y - h; }
    if x < l { x = l; }
    if y < t { y = t; }
    if x + w > r { x = r - w; }
    if y + h > b { y = b - h; }
    (x, y)
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
