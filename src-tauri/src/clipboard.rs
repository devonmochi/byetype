use arboard::Clipboard;
use std::process::Command;

/// Activate a named application on macOS.
#[cfg(target_os = "macos")]
fn activate_app(app_name: &str) {
    let _ = Command::new("osascript")
        .arg("-e")
        .arg(format!("tell application \"{}\" to activate", app_name))
        .output();
    // Give the app time to come to front
    std::thread::sleep(std::time::Duration::from_millis(100));
}

pub fn paste_text(text: &str, front_app: Option<String>) -> Result<(), String> {
    let mut clipboard = Clipboard::new()
        .map_err(|e| format!("Failed to access clipboard: {}", e))?;
    clipboard.set_text(text)
        .map_err(|e| format!("Failed to write to clipboard: {}", e))?;

    #[cfg(target_os = "macos")]
    {
        // Activate the original frontmost app before pasting
        if let Some(app_name) = front_app {
            activate_app(&app_name);
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
        Command::new("osascript")
            .arg("-e")
            .arg("tell application \"System Events\" to keystroke \"v\" using command down")
            .output()
            .map_err(|e| format!("Failed to simulate paste: {}", e))?;
    }

    #[cfg(target_os = "windows")]
    {
        let _ = front_app; // unused on Windows
        Command::new("powershell")
            .arg("-command")
            .arg("Add-Type -AssemblyName System.Windows.Forms; [System.Windows.Forms.SendKeys]::SendWait('^v')")
            .output()
            .map_err(|e| format!("Failed to simulate paste: {}", e))?;
    }

    Ok(())
}
