use arboard::Clipboard;

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
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
        SendInput,
        INPUT,
        INPUT_0,
        INPUT_KEYBOARD,
        KEYBDINPUT,
        KEYEVENTF_KEYUP,
        VK_CONTROL,
        VK_V,
    };

    pub fn simulate_paste() -> Result<(), String> {
        let vk_control = u16::try_from(VK_CONTROL)
            .map_err(|_| "Invalid VK_CONTROL value".to_string())?;
        let vk_v = u16::try_from(VK_V)
            .map_err(|_| "Invalid VK_V value".to_string())?;

        let inputs = [
            keyboard_input(vk_control, 0),
            keyboard_input(vk_v, 0),
            keyboard_input(vk_v, KEYEVENTF_KEYUP),
            keyboard_input(vk_control, KEYEVENTF_KEYUP),
        ];

        let sent = unsafe {
            SendInput(
                inputs.len() as u32,
                inputs.as_ptr(),
                std::mem::size_of::<INPUT>() as i32,
            )
        };

        if sent != inputs.len() as u32 {
            return Err(format!(
                "Failed to simulate paste: SendInput sent {}/{} events",
                sent,
                inputs.len()
            ));
        }

        Ok(())
    }

    fn keyboard_input(vk: u16, flags: u32) -> INPUT {
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: vk,
                    wScan: 0,
                    dwFlags: flags,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        }
    }
}

pub fn paste_text(text: &str) -> Result<(), String> {
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

    Ok(())
}
