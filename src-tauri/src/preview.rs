use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use tauri::{AppHandle, Emitter, Listener, Manager, WebviewUrl, WebviewWindowBuilder};

static PINNED: AtomicBool = AtomicBool::new(false);
/// Epoch millis when the preview window was created — ignore blur within grace period
static CREATED_AT: AtomicU64 = AtomicU64::new(0);
const BLUR_GRACE_MS: u128 = 800;
/// blur 监听器是否已注册(整个进程生命周期内只注册一次,避免复用窗口时叠加)
static BLUR_HANDLER_REGISTERED: AtomicBool = AtomicBool::new(false);

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
    // 每次新预览重置 pinned 状态
    PINNED.store(false, Ordering::Relaxed);

    // 按文本计算尺寸
    let line_count = text.lines().count().max(3).min(20);
    let max_line_len = text.lines().map(|l| l.len()).max().unwrap_or(40);
    let width = (max_line_len as f64 * 8.0 + 80.0).clamp(320.0, 600.0);
    let height = (line_count as f64 * 22.0 + 140.0).clamp(180.0, 460.0);

    // 优先复用已预热的窗口;否则新建
    let window = if let Some(existing) = app.get_webview_window("preview") {
        // 复用:按文本调整尺寸
        let _ = existing.set_size(tauri::LogicalSize::new(width, height));
        let _ = existing.center();
        existing
    } else {
        // 回退:新建窗口(退化到旧行为)
        WebviewWindowBuilder::new(app, "preview", WebviewUrl::App("preview.html".into()))
            .title("ByeType Preview")
            .inner_size(width, height)
            .resizable(true)
            .decorations(false)
            .always_on_top(true)
            .center()
            .visible(false)
            .build()
            .map_err(|e| format!("Create preview window failed: {}", e))?
    };

    // 显示策略:为避免「窗口先弹出再被新文本覆盖」造成首次内容闪错,
    // 必须等前端 setText 完成后再 window.show()。
    //
    // 两条触发路径:
    //   A. 冷启动:前端 mount 后 emit `preview-ready`,后端再 emit text;
    //      前端 setText 后 emit `preview-text-applied`,后端收到后才 show。
    //   B. 热复用:前端 listener 已注册,后端立即 emit text 即可被收到;
    //      同样靠 `preview-text-applied` 回执触发 show。
    //
    // 兜底:若 200ms 内未收到回执(前端崩溃/异常),强制 show,避免窗口永远不可见。
    let text_clone_for_ready = text.to_string();
    let window_for_ready = window.clone();
    window.once("preview-ready", move |_| {
        let _ = window_for_ready.emit("preview-text", &text_clone_for_ready);
    });

    let window_for_applied = window.clone();
    let shown = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let shown_for_applied = shown.clone();
    window.once("preview-text-applied", move |_| {
        if shown_for_applied
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
        {
            let _ = window_for_applied.show();
        }
    });

    // 立即 emit 一次:若窗口是预热的且 React 已 mount,此次 emit 会被前端立刻接收
    let _ = window.emit("preview-text", text);

    // 兜底超时:防止前端无回执时窗口永远不可见
    let window_for_fallback = window.clone();
    let shown_for_fallback = shown.clone();
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(200));
        if shown_for_fallback
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
        {
            let _ = window_for_fallback.show();
        }
    });

    // 记录创建时间用于 blur 宽限期
    CREATED_AT.store(now_ms(), Ordering::Relaxed);

    // blur 关闭事件:首次 show 注册一次,后续复用窗口跳过(避免监听器叠加)
    if BLUR_HANDLER_REGISTERED
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_ok()
    {
        let app_handle = app.clone();
        window.on_window_event(move |event| {
            match event {
                tauri::WindowEvent::Focused(false) => {
                    if PINNED.load(Ordering::Relaxed) {
                        return;
                    }
                    let age = now_ms().saturating_sub(CREATED_AT.load(Ordering::Relaxed));
                    if (age as u128) < BLUR_GRACE_MS {
                        return;
                    }
                    if let Some(w) = app_handle.get_webview_window("preview") {
                        let _ = w.close();
                    }
                }
                tauri::WindowEvent::Destroyed => {
                    // 窗口被销毁后,下次 show 需要重新注册监听器
                    BLUR_HANDLER_REGISTERED.store(false, Ordering::SeqCst);
                }
                _ => {}
            }
        });
    }

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
