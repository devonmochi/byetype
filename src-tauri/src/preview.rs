use tauri::{AppHandle, Emitter, Listener, Manager, WebviewUrl, WebviewWindowBuilder};

pub fn show(app: &AppHandle, text: &str) -> Result<(), String> {
    // Close existing preview window if any
    if let Some(window) = app.get_webview_window("preview") {
        let _ = window.close();
    }

    // Calculate window size based on text
    let line_count = text.lines().count().max(3).min(20);
    let max_line_len = text.lines().map(|l| l.len()).max().unwrap_or(40);
    let width = (max_line_len as f64 * 8.0 + 80.0).clamp(320.0, 600.0);
    let height = (line_count as f64 * 22.0 + 80.0).clamp(120.0, 400.0);

    let window = WebviewWindowBuilder::new(app, "preview", WebviewUrl::App("preview.html".into()))
        .title("ByeType Preview")
        .inner_size(width, height)
        .resizable(true)
        .decorations(false)
        .always_on_top(true)
        .center()
        .visible(false)
        .build()
        .map_err(|e| format!("Create preview window failed: {}", e))?;

    // Send text to frontend once it's ready
    let text_clone = text.to_string();
    let window_clone = window.clone();
    window.once("preview-ready", move |_| {
        let _ = window_clone.emit("preview-text", &text_clone);
        let _ = window_clone.show();
    });

    // Close window on blur (focus lost)
    let app_handle = app.clone();
    window.on_window_event(move |event| {
        if let tauri::WindowEvent::Focused(false) = event {
            if let Some(w) = app_handle.get_webview_window("preview") {
                let _ = w.close();
            }
        }
    });

    Ok(())
}
