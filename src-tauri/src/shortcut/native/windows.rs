//! Windows native `AltRight` backend.
//!
//! Raw Input is preferred (a hidden message window registered with
//! `RIDEV_INPUTSINK`, so events arrive even when ByeType is in the background).
//! When Raw Input cannot be used safely we fall back to a `WH_KEYBOARD_LL`
//! low-level keyboard hook. Only one backend runs at a time.
//!
//! Only the physical right Alt key is ever converted into an application event.
//! Other keys are ignored at the earliest possible point and are never logged,
//! buffered or forwarded.

use std::sync::{mpsc, Mutex, OnceLock};
use std::thread::JoinHandle;

use windows_sys::Win32::Foundation::{GetLastError, ERROR_CLASS_ALREADY_EXISTS, HWND, LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::Threading::GetCurrentThreadId;
use windows_sys::Win32::UI::Input::{
    GetRawInputData, GetRegisteredRawInputDevices, RegisterRawInputDevices, HRAWINPUT, RAWINPUT,
    RAWINPUTDEVICE, RAWINPUTHEADER, RID_INPUT, RIDEV_INPUTSINK, RIDEV_REMOVE, RIM_TYPEKEYBOARD,
};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{VK_LMENU, VK_MENU, VK_RMENU};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetMessageW,
    KBDLLHOOKSTRUCT, LLKHF_EXTENDED, LLKHF_INJECTED, LLKHF_LOWER_IL_INJECTED, MSG,
    PeekMessageW, PM_NOREMOVE, PostThreadMessageW, RegisterClassW, SetWindowsHookExW,
    TranslateMessage, UnhookWindowsHookEx, WH_KEYBOARD_LL, WM_APP, WM_KEYDOWN, WM_KEYUP, WM_QUIT,
    WM_SYSKEYDOWN, WM_SYSKEYUP, WNDCLASSW, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_POPUP,
};

use super::{make_backend, AltRightBackend, BackendKind, EventSink};

/// Raw Input keyboard flag: the key is an extended (E0-prefixed) key. Physical
/// right Alt is delivered as scan code 0x38 with this prefix.
const RI_KEY_BREAK: u16 = 0x01;
const RI_KEY_E0: u16 = 0x02;

/// Physical scan code of the Alt key (left and right share it; the E0 prefix
/// distinguishes the right one).
const SCANCODE_ALT: u16 = 0x38;

const RAW_INPUT_USAGE_PAGE_GENERIC: u16 = 0x01;
const RAW_INPUT_USAGE_KEYBOARD: u16 = 0x06;

/// Hidden window class name.
const WINDOW_CLASS: &str = "ByeTypeAltRightNative";
const WINDOW_TITLE: &str = "ByeTypeAltRightNative";

/// Message used to ask the listener thread to stop pumping.
const WM_ALT_RIGHT_STOP: u32 = WM_APP + 0x51;

static CALLBACK: OnceLock<Mutex<Option<EventSink>>> = OnceLock::new();

fn callback_slot() -> &'static Mutex<Option<EventSink>> {
    CALLBACK.get_or_init(|| Mutex::new(None))
}

fn set_callback(sink: EventSink) {
    *callback_slot().lock().unwrap_or_else(|e| e.into_inner()) = Some(sink);
}

fn clear_callback() {
    *callback_slot().lock().unwrap_or_else(|e| e.into_inner()) = None;
}

fn invoke_callback(pressed: bool) {
    let sink = callback_slot()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone();
    if let Some(sink) = sink {
        sink(pressed);
    }
}

fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

/// The process may only have one Raw Input target per device class. This check
/// reports whether a keyboard Raw Input registration already exists inside this
/// process (Tauri/WebView/winit/plugins). When it does, we must not silently
/// steal or overwrite it.
unsafe fn raw_input_keyboard_registered() -> bool {
    let mut devices: [RAWINPUTDEVICE; 32] = std::mem::zeroed();
    let mut count: u32 = devices.len() as u32;
    let result = GetRegisteredRawInputDevices(
        devices.as_mut_ptr(),
        &mut count,
        std::mem::size_of::<RAWINPUTDEVICE>() as u32,
    );
    if result == u32::MAX {
        // Cannot determine ownership; attempt registration and let it fail.
        return false;
    }
    devices[..count as usize].iter().any(|device| {
        device.usUsagePage == RAW_INPUT_USAGE_PAGE_GENERIC
            && device.usUsage == RAW_INPUT_USAGE_KEYBOARD
    })
}

pub fn start(sink: EventSink) -> Result<AltRightBackend, String> {
    if !unsafe { raw_input_keyboard_registered() } {
        match start_raw_input(sink.clone()) {
            Ok(backend) => return Ok(backend),
            Err(error) => {
                eprintln!("[shortcut] Raw Input 不可用（{error}），回退 WH_KEYBOARD_LL");
            }
        }
    } else {
        eprintln!("[shortcut] 进程内已存在键盘 Raw Input 注册，改用 WH_KEYBOARD_LL");
    }

    start_low_level_hook(sink).map_err(|hook_error| {
        format!("ALT_RIGHT_BACKEND_FAILED: Raw Input 与 Hook 均失败: {hook_error}")
    })
}

// ==================== Raw Input ====================

unsafe extern "system" fn raw_input_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if msg == windows_sys::Win32::UI::WindowsAndMessaging::WM_INPUT {
        handle_raw_input(lparam);
    }
    if msg == WM_ALT_RIGHT_STOP {
        windows_sys::Win32::UI::WindowsAndMessaging::PostQuitMessage(0);
        return 0;
    }
    DefWindowProcW(hwnd, msg, wparam, lparam)
}

unsafe fn handle_raw_input(lparam: LPARAM) {
    let mut raw: RAWINPUT = std::mem::zeroed();
    let mut size = std::mem::size_of::<RAWINPUT>() as u32;
    let read = GetRawInputData(
        lparam as HRAWINPUT,
        RID_INPUT,
        (&mut raw as *mut RAWINPUT).cast(),
        &mut size,
        std::mem::size_of::<RAWINPUTHEADER>() as u32,
    );
    if read == u32::MAX {
        return;
    }
    if raw.header.dwType != RIM_TYPEKEYBOARD {
        return;
    }
    let keyboard = raw.data.keyboard;
    if !raw_input_is_right_alt(&keyboard) {
        return;
    }
    let pressed = (keyboard.Flags & RI_KEY_BREAK) == 0;
    invoke_callback(pressed);
}

/// Physical right Alt is defined by the scan code plus the E0 prefix. Relying
/// on the virtual key alone is unreliable: drivers report either `VK_MENU`
/// (0x12) or the extended `VK_RMENU` (0xA5).
fn raw_input_is_right_alt(keyboard: &windows_sys::Win32::UI::Input::RAWKEYBOARD) -> bool {
    if keyboard.MakeCode == SCANCODE_ALT && (keyboard.Flags & RI_KEY_E0) != 0 {
        return true;
    }
    keyboard.VKey == VK_RMENU
}

fn start_raw_input(sink: EventSink) -> Result<AltRightBackend, String> {
    let (ready_tx, ready_rx) = mpsc::channel::<Result<u32, String>>();

    let handle = std::thread::Builder::new()
        .name("byetype-altright-rawinput".to_string())
        .spawn(move || {
            unsafe {
                set_callback(sink);
                let instance = GetModuleHandleW(std::ptr::null());
                let class_name = wide(WINDOW_CLASS);
                let window_title = wide(WINDOW_TITLE);
                let wnd_class = WNDCLASSW {
                    style: 0,
                    lpfnWndProc: Some(raw_input_wnd_proc),
                    cbClsExtra: 0,
                    cbWndExtra: 0,
                    hInstance: instance,
                    hIcon: std::ptr::null_mut(),
                    hCursor: std::ptr::null_mut(),
                    hbrBackground: std::ptr::null_mut(),
                    lpszMenuName: std::ptr::null(),
                    lpszClassName: class_name.as_ptr(),
                };
                let atom = RegisterClassW(&wnd_class);
                if atom == 0 && GetLastError() != ERROR_CLASS_ALREADY_EXISTS {
                    ready_tx
                        .send(Err(format!("RegisterClassW 失败（错误码 {}）", GetLastError())))
                        .ok();
                    clear_callback();
                    return;
                }

                let hwnd = CreateWindowExW(
                    WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
                    class_name.as_ptr(),
                    window_title.as_ptr(),
                    WS_POPUP,
                    0,
                    0,
                    0,
                    0,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    instance,
                    std::ptr::null(),
                );
                if hwnd.is_null() {
                    ready_tx.send(Err("CreateWindowExW 失败".to_string())).ok();
                    clear_callback();
                    return;
                }

                let device = RAWINPUTDEVICE {
                    usUsagePage: RAW_INPUT_USAGE_PAGE_GENERIC,
                    usUsage: RAW_INPUT_USAGE_KEYBOARD,
                    // No RIDEV_NOLEGACY: normal keyboard messages must keep flowing.
                    dwFlags: RIDEV_INPUTSINK,
                    hwndTarget: hwnd,
                };
                if RegisterRawInputDevices(
                    &device,
                    1,
                    std::mem::size_of::<RAWINPUTDEVICE>() as u32,
                ) == 0
                {
                    DestroyWindow(hwnd);
                    ready_tx
                        .send(Err("RegisterRawInputDevices 失败".to_string()))
                        .ok();
                    clear_callback();
                    return;
                }

                ready_tx.send(Ok(GetCurrentThreadId())).ok();

                let mut msg: MSG = std::mem::zeroed();
                while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {
                    TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }

                // Unregister only our own target, then destroy the window.
                let remove = RAWINPUTDEVICE {
                    usUsagePage: RAW_INPUT_USAGE_PAGE_GENERIC,
                    usUsage: RAW_INPUT_USAGE_KEYBOARD,
                    dwFlags: RIDEV_REMOVE,
                    hwndTarget: std::ptr::null_mut(),
                };
                RegisterRawInputDevices(
                    &remove,
                    1,
                    std::mem::size_of::<RAWINPUTDEVICE>() as u32,
                );
                DestroyWindow(hwnd);
                clear_callback();
            }
        })
        .map_err(|e| format!("无法创建 Raw Input 线程: {e}"))?;

    let thread_id = match ready_rx.recv() {
        Ok(Ok(thread_id)) => thread_id,
        Ok(Err(error)) => {
            let _ = handle.join();
            return Err(error);
        }
        Err(_) => {
            let _ = handle.join();
            return Err("Raw Input 线程初始化中断".to_string());
        }
    };

    Ok(make_backend(
        BackendKind::RawInput,
        stop_thread(handle, thread_id),
    ))
}

// ==================== Low level keyboard hook ====================

unsafe extern "system" fn keyboard_hook_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code >= 0 {
        let message = wparam as u32;
        if matches!(message, WM_KEYDOWN | WM_SYSKEYDOWN | WM_KEYUP | WM_SYSKEYUP) {
            let keyboard = &*(lparam as *const KBDLLHOOKSTRUCT);
            let injected = keyboard.flags
                & (LLKHF_INJECTED | LLKHF_LOWER_IL_INJECTED)
                != 0;
            if !injected && hook_is_right_alt(keyboard) {
                let pressed = matches!(message, WM_KEYDOWN | WM_SYSKEYDOWN);
                invoke_callback(pressed);
            }
        }
    }
    CallNextHookEx(
        std::ptr::null_mut(),
        code,
        wparam,
        lparam,
    )
}

fn start_low_level_hook(sink: EventSink) -> Result<AltRightBackend, String> {
    let (ready_tx, ready_rx) = mpsc::channel::<Result<u32, String>>();

    let handle = std::thread::Builder::new()
        .name("byetype-altright-hook".to_string())
        .spawn(move || {
            unsafe {
                set_callback(sink);
                let module = GetModuleHandleW(std::ptr::null());
                let hook = SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard_hook_proc), module, 0);
                if hook.is_null() {
                    ready_tx
                        .send(Err("SetWindowsHookExW(WH_KEYBOARD_LL) 失败".to_string()))
                        .ok();
                    clear_callback();
                    return;
                }

                // Force creation of this thread's message queue so that a stop
                // message posted immediately after `ready` is not lost.
                let mut msg: MSG = std::mem::zeroed();
                PeekMessageW(&mut msg, std::ptr::null_mut(), 0, 0, PM_NOREMOVE);

                ready_tx.send(Ok(GetCurrentThreadId())).ok();

                while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {
                    TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }

                UnhookWindowsHookEx(hook);
                clear_callback();
            }
        })
        .map_err(|e| format!("无法创建 Hook 线程: {e}"))?;

    let thread_id = match ready_rx.recv() {
        Ok(Ok(thread_id)) => thread_id,
        Ok(Err(error)) => {
            let _ = handle.join();
            return Err(error);
        }
        Err(_) => {
            let _ = handle.join();
            return Err("Hook 线程初始化中断".to_string());
        }
    };

    Ok(make_backend(
        BackendKind::LowLevelHook,
        stop_thread(handle, thread_id),
    ))
}

fn stop_thread(handle: JoinHandle<()>, thread_id: u32) -> Box<dyn FnOnce() + Send> {
    Box::new(move || {
        unsafe {
            PostThreadMessageW(thread_id, WM_QUIT, 0, 0);
        }
        let _ = handle.join();
    })
}

/// Low-level hook right-Alt detection. Uses the physical scan code + extended
/// flag first, then falls back to the virtual key (`VK_MENU`+E0 or `VK_RMENU`).
/// Left Alt (`VK_MENU`/`VK_LMENU` without the extended flag) is never matched.
fn hook_is_right_alt(keyboard: &KBDLLHOOKSTRUCT) -> bool {
    if keyboard.scanCode == SCANCODE_ALT as u32
        && (keyboard.flags & LLKHF_EXTENDED) != 0
    {
        return true;
    }
    keyboard.vkCode == VK_RMENU as u32
        || (keyboard.vkCode == VK_MENU as u32 && (keyboard.flags & LLKHF_EXTENDED) != 0)
}

#[allow(dead_code)]
fn is_left_alt_vk(vk: u32) -> bool {
    vk == VK_LMENU as u32
}
