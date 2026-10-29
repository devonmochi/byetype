//! Windows 物理右 Alt 监听。
//!
//! 使用低级键盘钩子接收后台按键，只识别右 Alt，不拦截或修改任何按键。
//! 回调只负责转发状态，录音和识别在调用方的工作线程执行。

use std::sync::{mpsc, Arc, Mutex, OnceLock};
use std::thread::JoinHandle;

use windows_sys::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::Threading::GetCurrentThreadId;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{VK_MENU, VK_RMENU};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, DispatchMessageW, GetMessageW, PeekMessageW, PostThreadMessageW,
    SetWindowsHookExW, TranslateMessage, UnhookWindowsHookEx, KBDLLHOOKSTRUCT, LLKHF_EXTENDED,
    LLKHF_INJECTED, LLKHF_LOWER_IL_INJECTED, MSG, PM_NOREMOVE, WH_KEYBOARD_LL, WM_KEYDOWN,
    WM_KEYUP, WM_QUIT, WM_SYSKEYDOWN, WM_SYSKEYUP,
};

pub type EventSink = Arc<dyn Fn(bool) + Send + Sync>;

static CALLBACK: OnceLock<Mutex<Option<EventSink>>> = OnceLock::new();

fn callback_slot() -> &'static Mutex<Option<EventSink>> {
    CALLBACK.get_or_init(|| Mutex::new(None))
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

fn is_right_alt(keyboard: &KBDLLHOOKSTRUCT) -> bool {
    // 左、右 Alt 通常共用扫描码 0x38，右 Alt 带 E0 扩展标志。
    // 某些驱动只提供虚拟键，因此再以 VK_RMENU 兜底。
    (keyboard.scanCode == 0x38 && (keyboard.flags & LLKHF_EXTENDED) != 0)
        || keyboard.vkCode == VK_RMENU as u32
        || (keyboard.vkCode == VK_MENU as u32 && (keyboard.flags & LLKHF_EXTENDED) != 0)
}

unsafe extern "system" fn keyboard_hook_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code >= 0 {
        let message = wparam as u32;
        if matches!(message, WM_KEYDOWN | WM_SYSKEYDOWN | WM_KEYUP | WM_SYSKEYUP) {
            let keyboard = &*(lparam as *const KBDLLHOOKSTRUCT);
            let injected = keyboard.flags & (LLKHF_INJECTED | LLKHF_LOWER_IL_INJECTED) != 0;
            if !injected && is_right_alt(keyboard) {
                let pressed = matches!(message, WM_KEYDOWN | WM_SYSKEYDOWN);
                invoke_callback(pressed);
            }
        }
    }
    CallNextHookEx(std::ptr::null_mut(), code, wparam, lparam)
}

pub struct NativeBackend {
    thread_id: u32,
    handle: Option<JoinHandle<()>>,
}

impl NativeBackend {
    pub fn start(sink: EventSink) -> Result<Self, String> {
        let (ready_tx, ready_rx) = mpsc::channel::<Result<u32, String>>();
        let handle = std::thread::Builder::new()
            .name("byetype-altright-hook".to_string())
            .spawn(move || unsafe {
                *callback_slot().lock().unwrap_or_else(|e| e.into_inner()) = Some(sink);
                let module = GetModuleHandleW(std::ptr::null());
                let hook = SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard_hook_proc), module, 0);
                if hook.is_null() {
                    let _ = ready_tx.send(Err("无法安装右 Alt 键盘监听".to_string()));
                    *callback_slot().lock().unwrap_or_else(|e| e.into_inner()) = None;
                    return;
                }

                // 先创建消息队列，确保监听启动后立即停止也不会丢失退出消息。
                let mut msg: MSG = std::mem::zeroed();
                PeekMessageW(&mut msg, std::ptr::null_mut(), 0, 0, PM_NOREMOVE);
                let _ = ready_tx.send(Ok(GetCurrentThreadId()));

                while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {
                    TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }

                UnhookWindowsHookEx(hook);
                *callback_slot().lock().unwrap_or_else(|e| e.into_inner()) = None;
            })
            .map_err(|e| format!("无法创建右 Alt 监听线程: {e}"))?;

        match ready_rx.recv() {
            Ok(Ok(thread_id)) => Ok(Self {
                thread_id,
                handle: Some(handle),
            }),
            Ok(Err(error)) => {
                let _ = handle.join();
                Err(error)
            }
            Err(_) => {
                let _ = handle.join();
                Err("右 Alt 监听初始化中断".to_string())
            }
        }
    }
}

impl Drop for NativeBackend {
    fn drop(&mut self) {
        unsafe {
            PostThreadMessageW(self.thread_id, WM_QUIT, 0, 0);
        }
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}
