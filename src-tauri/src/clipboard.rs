use arboard::Clipboard;
use std::process::Command;

pub fn paste_text(text: &str) -> Result<(), String> {
    let mut clipboard = Clipboard::new()
        .map_err(|e| format!("Failed to access clipboard: {}", e))?;
    clipboard.set_text(text)
        .map_err(|e| format!("Failed to write to clipboard: {}", e))?;

    #[cfg(target_os = "macos")]
    {
        std::thread::sleep(std::time::Duration::from_millis(50));
        Command::new("osascript")
            .arg("-e")
            .arg("tell application \"System Events\" to keystroke \"v\" using command down")
            .output()
            .map_err(|e| format!("Failed to simulate paste: {}", e))?;
    }

    #[cfg(target_os = "windows")]
    {
        Command::new("powershell")
            .arg("-command")
            .arg("Add-Type -AssemblyName System.Windows.Forms; [System.Windows.Forms.SendKeys]::SendWait('^v')")
            .output()
            .map_err(|e| format!("Failed to simulate paste: {}", e))?;
    }

    Ok(())
}
