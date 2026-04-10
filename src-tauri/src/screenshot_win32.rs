//! Native Win32 screenshot region selection.
//! Uses a real Win32 layered window instead of WebView2 to avoid mouse event issues.

use std::sync::Mutex;
use crate::task::ScreenshotCrop;

struct State {
    dragging: bool,
    start_x: i32,
    start_y: i32,
    cur_x: i32,
    cur_y: i32,
    result: Option<ScreenshotCrop>,
    done: bool,
}

static STATE: Mutex<State> = Mutex::new(State {
    dragging: false,
    start_x: 0,
    start_y: 0,
    cur_x: 0,
    cur_y: 0,
    result: None,
    done: false,
});

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

unsafe extern "system" fn wndproc(
    hwnd: windows_sys::Win32::Foundation::HWND,
    msg: u32,
    wparam: windows_sys::Win32::Foundation::WPARAM,
    lparam: windows_sys::Win32::Foundation::LPARAM,
) -> windows_sys::Win32::Foundation::LRESULT {
    use windows_sys::Win32::UI::WindowsAndMessaging::*;
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::*;
    use windows_sys::Win32::Graphics::Gdi::*;
    use windows_sys::Win32::Foundation::*;

    match msg {
        WM_LBUTTONDOWN => {
            let x = (lparam & 0xFFFF) as i16 as i32;
            let y = ((lparam >> 16) & 0xFFFF) as i16 as i32;
            SetCapture(hwnd);
            if let Ok(mut state) = STATE.lock() {
                state.dragging = true;
                state.start_x = x;
                state.start_y = y;
                state.cur_x = x;
                state.cur_y = y;
            }
            0
        }
        WM_MOUSEMOVE => {
            let x = (lparam & 0xFFFF) as i16 as i32;
            let y = ((lparam >> 16) & 0xFFFF) as i16 as i32;
            let is_dragging = if let Ok(mut state) = STATE.lock() {
                if state.dragging {
                    state.cur_x = x;
                    state.cur_y = y;
                    true
                } else {
                    false
                }
            } else {
                false
            };
            if is_dragging {
                InvalidateRect(hwnd, std::ptr::null(), 0);
            }
            0
        }
        WM_LBUTTONUP => {
            let x = (lparam & 0xFFFF) as i16 as i32;
            let y = ((lparam >> 16) & 0xFFFF) as i16 as i32;
            ReleaseCapture();
            if let Ok(mut state) = STATE.lock() {
                if state.dragging {
                    state.dragging = false;
                    let sx = state.start_x.min(x);
                    let sy = state.start_y.min(y);
                    let w = (state.start_x - x).unsigned_abs();
                    let h = (state.start_y - y).unsigned_abs();
                    if w > 5 && h > 5 {
                        state.result = Some(ScreenshotCrop {
                            x: sx as u32,
                            y: sy as u32,
                            w,
                            h,
                        });
                    } else {
                    }
                    state.done = true;
                }
            }
            PostQuitMessage(0);
            0
        }
        WM_KEYDOWN => {
            if wparam == VK_ESCAPE as usize {
                if let Ok(mut state) = STATE.lock() {
                    state.done = true;
                    state.result = None;
                }
                PostQuitMessage(0);
            }
            0
        }
        WM_PAINT => {
            let mut ps = std::mem::zeroed::<PAINTSTRUCT>();
            let hdc = BeginPaint(hwnd, &mut ps);

            // Fill entire area with black
            let mut client_rect = std::mem::zeroed::<RECT>();
            GetClientRect(hwnd, &mut client_rect);
            let black_brush = CreateSolidBrush(0x00000000);
            FillRect(hdc, &client_rect, black_brush);
            DeleteObject(black_brush);

            // Draw selection rectangle if dragging
            let (draw, sx, sy, ex, ey) = if let Ok(state) = STATE.lock() {
                if state.dragging {
                    (
                        true,
                        state.start_x.min(state.cur_x),
                        state.start_y.min(state.cur_y),
                        state.start_x.max(state.cur_x),
                        state.start_y.max(state.cur_y),
                    )
                } else {
                    (false, 0, 0, 0, 0)
                }
            } else {
                (false, 0, 0, 0, 0)
            };

            if draw {
                // Clear selection area (make it "transparent" by not painting it dark)
                // Since layered alpha applies uniformly, we paint selection area lighter
                let light_brush = CreateSolidBrush(0x00404040);
                let sel_rect = RECT { left: sx, top: sy, right: ex, bottom: ey };
                FillRect(hdc, &sel_rect, light_brush);
                DeleteObject(light_brush);

                // Draw white border
                let pen = CreatePen(PS_SOLID as i32, 2, 0x00FFFFFF);
                let old_pen = SelectObject(hdc, pen);
                let null_brush = GetStockObject(NULL_BRUSH);
                let old_brush = SelectObject(hdc, null_brush);
                Rectangle(hdc, sx, sy, ex, ey);
                SelectObject(hdc, old_pen);
                SelectObject(hdc, old_brush);
                DeleteObject(pen);
            }

            EndPaint(hwnd, &ps);
            0
        }
        WM_RBUTTONDOWN => {
            // Right click cancels
            if let Ok(mut state) = STATE.lock() {
                state.done = true;
                state.result = None;
            }
            PostQuitMessage(0);
            0
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

/// Runs a native Win32 overlay for region selection.
/// Blocks until user completes selection or cancels.
pub fn select_region(mon_x: i32, mon_y: i32, mon_w: i32, mon_h: i32) -> Option<ScreenshotCrop> {
    use windows_sys::Win32::UI::WindowsAndMessaging::*;
    use windows_sys::Win32::Graphics::Gdi::*;
    use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;

    // Reset state
    if let Ok(mut state) = STATE.lock() {
        state.dragging = false;
        state.start_x = 0;
        state.start_y = 0;
        state.cur_x = 0;
        state.cur_y = 0;
        state.result = None;
        state.done = false;
    }

    unsafe {
        let class_name = wide("ByeTypeScreenshot");
        let hinstance = GetModuleHandleW(std::ptr::null());

        let wc = WNDCLASSW {
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(wndproc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: hinstance,
            hIcon: std::ptr::null_mut(),
            hCursor: LoadCursorW(std::ptr::null_mut(), IDC_CROSS),
            hbrBackground: std::ptr::null_mut(),
            lpszMenuName: std::ptr::null(),
            lpszClassName: class_name.as_ptr(),
        };

        RegisterClassW(&wc);

        let hwnd = CreateWindowExW(
            WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_NOACTIVATE,
            class_name.as_ptr(),
            std::ptr::null(),
            WS_POPUP | WS_VISIBLE,
            mon_x,
            mon_y,
            mon_w,
            mon_h,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            hinstance,
            std::ptr::null(),
        );

        if hwnd.is_null() {
            UnregisterClassW(class_name.as_ptr(), hinstance);
            return None;
        }

        // Semi-transparent dark overlay (alpha ~80/255 ≈ 31%)
        SetLayeredWindowAttributes(hwnd, 0, 80, LWA_ALPHA);
        SetForegroundWindow(hwnd);

        // Message loop
        let mut msg = std::mem::zeroed::<MSG>();
        while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }

        DestroyWindow(hwnd);
        UnregisterClassW(class_name.as_ptr(), hinstance);

        let result = if let Ok(state) = STATE.lock() {
            state.result.clone()
        } else {
            None
        };

        result
    }
}
