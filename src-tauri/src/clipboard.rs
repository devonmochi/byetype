use arboard::Clipboard;

use std::borrow::Cow;

/// 剪贴板原内容快照，用于 paste_text 完成后还原。
/// 仅支持 arboard 能稳定读写的两种类型；文件引用 / 富文本 / 空 → None。
enum ClipboardSnapshot {
    Text(String),
    Image(arboard::ImageData<'static>),
    None,
}

fn snapshot_clipboard() -> ClipboardSnapshot {
    let mut clipboard = match Clipboard::new() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("[clipboard] snapshot: failed to open clipboard: {}", e);
            return ClipboardSnapshot::None;
        }
    };

    if let Ok(text) = clipboard.get_text() {
        return ClipboardSnapshot::Text(text);
    }

    match clipboard.get_image() {
        Ok(img) => ClipboardSnapshot::Image(arboard::ImageData {
            width: img.width,
            height: img.height,
            bytes: Cow::Owned(img.bytes.into_owned()),
        }),
        Err(_) => ClipboardSnapshot::None,
    }
}

fn restore_clipboard(snapshot: ClipboardSnapshot) {
    if matches!(snapshot, ClipboardSnapshot::None) {
        return;
    }
    let mut clipboard = match Clipboard::new() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("[clipboard] restore: failed to open clipboard: {}", e);
            return;
        }
    };
    let result = match snapshot {
        ClipboardSnapshot::Text(s) => clipboard.set_text(s).map_err(|e| format!("set_text: {}", e)),
        ClipboardSnapshot::Image(i) => clipboard.set_image(i).map_err(|e| format!("set_image: {}", e)),
        ClipboardSnapshot::None => Ok(()),
    };
    if let Err(e) = result {
        eprintln!("[clipboard] restore failed: {}", e);
    }
}

#[cfg(target_os = "macos")]
mod macos {
    use core_graphics::event::{CGEvent, CGEventFlags, CGKeyCode, CGEventTapLocation};
    use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};
    use std::ffi::c_void;

    const KEY_V: CGKeyCode = 9;

    extern "C" {
        fn CFStringCreateWithCString(
            alloc: *const c_void,
            c_str: *const u8,
            encoding: u32,
        ) -> *const c_void;
        fn CFDictionaryCreate(
            allocator: *const c_void,
            keys: *const *const c_void,
            values: *const *const c_void,
            num_values: isize,
            key_callbacks: *const c_void,
            value_callbacks: *const c_void,
        ) -> *const c_void;
        fn CFRelease(cf: *const c_void);
        static kCFTypeDictionaryKeyCallBacks: c_void;
        static kCFTypeDictionaryValueCallBacks: c_void;
        static kCFBooleanTrue: *const c_void;
    }

    #[link(name = "ApplicationServices", kind = "framework")]
    extern "C" {
        fn AXIsProcessTrustedWithOptions(options: *const c_void) -> bool;
    }

    pub fn ensure_accessibility() -> bool {
        unsafe {
            let key = CFStringCreateWithCString(
                std::ptr::null(),
                b"AXTrustedCheckOptionPrompt\0".as_ptr(),
                0x08000100,
            );
            let keys = [key];
            let values = [kCFBooleanTrue];
            let options = CFDictionaryCreate(
                std::ptr::null(),
                keys.as_ptr(),
                values.as_ptr(),
                1,
                &kCFTypeDictionaryKeyCallBacks as *const _ as *const c_void,
                &kCFTypeDictionaryValueCallBacks as *const _ as *const c_void,
            );
            let trusted = AXIsProcessTrustedWithOptions(options);
            CFRelease(options);
            CFRelease(key);
            trusted
        }
    }

    pub fn simulate_paste() -> Result<(), String> {
        let source = CGEventSource::new(CGEventSourceStateID::CombinedSessionState)
            .map_err(|_| "Failed to create CGEventSource".to_string())?;

        let key_down = CGEvent::new_keyboard_event(source.clone(), KEY_V, true)
            .map_err(|_| "Failed to create key down event".to_string())?;
        key_down.set_flags(CGEventFlags::CGEventFlagCommand);

        let key_up = CGEvent::new_keyboard_event(source, KEY_V, false)
            .map_err(|_| "Failed to create key up event".to_string())?;
        key_up.set_flags(CGEventFlags::CGEventFlagCommand);

        key_down.post(CGEventTapLocation::HID);
        key_up.post(CGEventTapLocation::HID);

        Ok(())
    }
}

#[cfg(target_os = "windows")]
mod windows {
    use enigo::{Enigo, Keyboard, Settings, Key, Direction};

    pub fn simulate_paste() -> Result<(), String> {
        let mut enigo = Enigo::new(&Settings::default())
            .map_err(|e| format!("Failed to create Enigo instance: {}", e))?;

        enigo.key(Key::Control, Direction::Press)
            .map_err(|e| format!("Failed to press Ctrl: {}", e))?;
        enigo.key(Key::Unicode('v'), Direction::Press)
            .map_err(|e| format!("Failed to press V: {}", e))?;
        enigo.key(Key::Unicode('v'), Direction::Release)
            .map_err(|e| format!("Failed to release V: {}", e))?;
        enigo.key(Key::Control, Direction::Release)
            .map_err(|e| format!("Failed to release Ctrl: {}", e))?;

        Ok(())
    }
}

pub fn paste_text(text: &str, overwrite_clipboard: bool) -> Result<(), String> {
    // OFF 模式：先快照原剪贴板，主流程结束后还原。
    let backup = if !overwrite_clipboard {
        snapshot_clipboard()
    } else {
        ClipboardSnapshot::None
    };

    let mut clipboard = Clipboard::new()
        .map_err(|e| format!("Failed to access clipboard: {}", e))?;
    clipboard.set_text(text)
        .map_err(|e| format!("Failed to write to clipboard: {}", e))?;

    #[cfg(target_os = "macos")]
    {
        if !macos::ensure_accessibility() {
            return Err("Accessibility permission not granted, please allow in System Settings".to_string());
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
        macos::simulate_paste()?;
    }

    #[cfg(target_os = "windows")]
    {
        windows::simulate_paste()?;
    }

    // 仅当备份非 None 时才执行还原；让目标应用先读完粘贴内容。
    if !matches!(backup, ClipboardSnapshot::None) {
        std::thread::sleep(std::time::Duration::from_millis(150));
        restore_clipboard(backup);
    }

    Ok(())
}
