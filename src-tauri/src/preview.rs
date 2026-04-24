use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use tauri::{AppHandle, Emitter, Listener, Manager, WebviewUrl, WebviewWindowBuilder};

static PINNED: AtomicBool = AtomicBool::new(false);
/// Epoch millis when the preview window was created — ignore blur within grace period
static CREATED_AT: AtomicU64 = AtomicU64::new(0);
const BLUR_GRACE_MS: u128 = 800;

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

#[tauri::command]
pub fn set_preview_pinned(pinned: bool) {
    PINNED.store(pinned, Ordering::Relaxed);
}

#[tauri::command]
pub fn close_preview_window(app: AppHandle) {
    if let Some(window) = app.get_webview_window("preview") {
        let _ = window.close();
    }
}

pub fn show(app: &AppHandle, text: &str) -> Result<(), String> {
    // Reset pinned state for each new preview
    PINNED.store(false, Ordering::Relaxed);

    // Close existing preview window if any
    if let Some(window) = app.get_webview_window("preview") {
        let _ = window.close();
    }

    // Calculate window size based on text
    let line_count = text.lines().count().max(3).min(20);
    let max_line_len = text.lines().map(|l| l.len()).max().unwrap_or(40);
    let width = (max_line_len as f64 * 8.0 + 80.0).clamp(320.0, 600.0);
    let height = (line_count as f64 * 22.0 + 140.0).clamp(180.0, 460.0);

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

    // Record creation time for blur grace period
    CREATED_AT.store(now_ms(), Ordering::Relaxed);

    // Close window on blur (focus lost) — only if not pinned and past grace period
    let app_handle = app.clone();
    window.on_window_event(move |event| {
        if let tauri::WindowEvent::Focused(false) = event {
            if PINNED.load(Ordering::Relaxed) {
                return;
            }
            // Skip blur events during grace period (window may not have focus yet)
            let age = now_ms().saturating_sub(CREATED_AT.load(Ordering::Relaxed));
            if (age as u128) < BLUR_GRACE_MS {
                return;
            }
            if let Some(w) = app_handle.get_webview_window("preview") {
                let _ = w.close();
            }
        }
    });

    Ok(())
}

/// 预热:提前创建一个隐藏的预览窗口,让 React bundle 后台加载。
///
/// 幂等 —— 若 preview 窗口已存在则直接返回。调用发生在 AI 调用开始时,
/// 利用 AI 等待时间掩盖 webview 冷启动开销。失败只打 log,不中断主流程
/// (后续 show() 会走创建分支,退化到旧行为)。
pub fn prewarm(app: &AppHandle) {
    // 幂等检查必须在主线程调度前做,避免重复分派
    if app.get_webview_window("preview").is_some() {
        return;
    }
    let app_cloned = app.clone();
    if let Err(e) = app.run_on_main_thread(move || {
        // 主线程上再次检查,防止调度延迟期间被重复派发
        if app_cloned.get_webview_window("preview").is_some() {
            return;
        }
        let result = WebviewWindowBuilder::new(
            &app_cloned,
            "preview",
            WebviewUrl::App("preview.html".into()),
        )
        .title("ByeType Preview")
        .inner_size(400.0, 300.0) // 占位尺寸,show() 时再按文本调整
        .resizable(true)
        .decorations(false)
        .always_on_top(true)
        .center()
        .visible(false)
        .build();
        if let Err(e) = result {
            eprintln!("[preview] prewarm failed: {}", e);
        }
    }) {
        eprintln!("[preview] prewarm dispatch failed: {}", e);
    }
}

/// 供失败路径调用:若存在 preview 窗口(可能是预热残留)则关闭。
pub fn close_if_exists(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("preview") {
        let _ = window.close();
    }
}
