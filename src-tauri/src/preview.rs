use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use tauri::{AppHandle, Emitter, Listener, Manager, WebviewUrl, WebviewWindowBuilder};

/// 光标与预览窗口之间的边距(逻辑像素/物理像素,取决于平台 set_position 分支)。
const CURSOR_OFFSET: f64 = 12.0;

static PINNED: AtomicBool = AtomicBool::new(false);
/// Epoch millis when the preview window was created — ignore blur within grace period
static CREATED_AT: AtomicU64 = AtomicU64::new(0);
const BLUR_GRACE_MS: u128 = 800;
/// blur 监听器是否已注册(整个进程生命周期内只注册一次,避免复用窗口时叠加)
static BLUR_HANDLER_REGISTERED: AtomicBool = AtomicBool::new(false);
/// 前端 React 是否已完成 mount 并注册 preview-text listener。
/// prewarm 可能只创建了 WebviewWindow，但 listener 还没 ready；此时不能按热复用处理。
static PREVIEW_READY: AtomicBool = AtomicBool::new(false);

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

    // 显示策略:为避免「窗口先弹出再被新文本覆盖」造成首次内容闪错,
    // 必须等前端 setText 完成后再 window.show()。前端用 flushSync 保证
    // setText commit 完成后才发 `preview-text-applied` 回执。
    //
    // 两条触发路径:
    //   A. 冷启动(新建窗口):前端 mount 后 emit `preview-ready`,后端再 emit text。
    //      preview-ready once handler 只在新建分支注册,避免热复用时累积。
    //   B. 热复用:前端 listener 已注册,后端立即 emit text 即可被收到。
    //
    // 兜底:若 200ms 内未收到回执(前端崩溃/异常),强制 show,避免窗口永远不可见。
    let shown = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));

    // 优先复用已预热的窗口;否则新建。
    // 注意:prewarm 可能只完成了 WebviewWindow 创建，React 还没 mount 完成。
    // 因此「窗口存在」不等于「前端 listener 已 ready」。
    let window = if let Some(existing) = app.get_webview_window("preview") {
        // 复用:按文本调整尺寸
        let _ = existing.set_size(tauri::LogicalSize::new(width, height));

        if !PREVIEW_READY.load(Ordering::SeqCst) {
            // 半热复用:窗口已存在但前端还没发 preview-ready。
            // 等 listener 注册完成后再补发文本，避免立即 emit 丢失后只显示空白兜底窗口。
            let text_for_ready = text.to_string();
            let window_for_ready = existing.clone();
            existing.once("preview-ready", move |_| {
                PREVIEW_READY.store(true, Ordering::SeqCst);
                let _ = window_for_ready.emit("preview-text", &text_for_ready);
            });
        }

        existing
    } else {
        // 新建路径:注册一次 preview-ready,等 React mount 后回推 text。
        PREVIEW_READY.store(false, Ordering::SeqCst);
        let built = WebviewWindowBuilder::new(app, "preview", WebviewUrl::App("preview.html".into()))
            .title("ByeType Preview")
            .inner_size(width, height)
            .resizable(true)
            .decorations(false)
            .always_on_top(true)
            .visible(false)
            .build()
            .map_err(|e| format!("Create preview window failed: {}", e))?;
        let text_for_ready = text.to_string();
        let window_for_ready = built.clone();
        built.once("preview-ready", move |_| {
            PREVIEW_READY.store(true, Ordering::SeqCst);
            let _ = window_for_ready.emit("preview-text", &text_for_ready);
        });
        built
    };

    // 定位到光标右下,超出屏幕则反向贴边——必须在 show() 之前完成
    position_near_cursor(&window, width, height);

    let window_for_applied = window.clone();
    let shown_for_applied = shown.clone();
    window.once("preview-text-applied", move |_| {
        if shown_for_applied
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
        {
            let _ = window_for_applied.show();
        }
    });

    // 立即 emit 一次:若窗口是预热的且 React 已 mount,此次 emit 会被前端立刻接收。
    // 冷启动场景前端 listener 还没注册,这次 emit 会丢失——由 preview-ready 路径兜底。
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
                    // 前端随窗口一起销毁,ready 状态失效;下次新建窗口须从 false 重新起算
                    PREVIEW_READY.store(false, Ordering::SeqCst);
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
        // 预热即开始 React 冷启动，ready 状态从 false 起算。
        PREVIEW_READY.store(false, Ordering::SeqCst);
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
        match result {
            Ok(window) => {
                // 关键:预热窗口的 React mount 完成会 emit preview-ready，
                // 这发生在 show() 之前。必须在此接住并记录，否则该信号无人接收，
                // show() 时既无法判断前端已 ready，再注册 once 也永不触发
                // (事件已过去)，最终只剩 200ms 兜底显示空白窗口。
                window.once("preview-ready", |_| {
                    PREVIEW_READY.store(true, Ordering::SeqCst);
                });
            }
            Err(e) => {
                eprintln!("[preview] prewarm failed: {}", e);
            }
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

/// 把窗口定位到光标右下偏移处,超出屏幕则反向贴边到左/上侧,
/// 保证窗口完整可见。读不到光标/屏幕信息时回退到 center()。
///
/// 坐标系约定:
/// - macOS: 光标 (CGEvent) 与 Monitor API 都用逻辑点,直接走 LogicalPosition。
/// - Windows: 光标 (GetCursorPos) 与 Monitor API 都用物理像素,走 PhysicalPosition。
/// 全程不做平台间换算,避免 DPI 误差。
fn position_near_cursor(window: &tauri::WebviewWindow, width: f64, height: f64) {
    let monitors = match window.available_monitors() {
        Ok(m) if !m.is_empty() => m,
        _ => { let _ = window.center(); return; }
    };

    #[cfg(target_os = "macos")]
    {
        let cursor = match macos_cursor() {
            Some(c) => c,
            None => { let _ = window.center(); return; }
        };
        // macOS: 光标与 monitor 都用逻辑点比较
        let monitor = monitors.iter().find(|m| {
            let s = m.scale_factor();
            let l = m.position().x as f64 / s;
            let t = m.position().y as f64 / s;
            let r = l + m.size().width as f64 / s;
            let b = t + m.size().height as f64 / s;
            cursor.0 >= l && cursor.0 < r && cursor.1 >= t && cursor.1 < b
        }).unwrap_or(&monitors[0]);

        let s = monitor.scale_factor();
        let l = monitor.position().x as f64 / s;
        let t = monitor.position().y as f64 / s;
        let r = l + monitor.size().width as f64 / s;
        let b = t + monitor.size().height as f64 / s;

        let (x, y) = pick_pos(cursor.0, cursor.1, width, height, l, t, r, b, CURSOR_OFFSET);
        let _ = window.set_position(tauri::Position::Logical(
            tauri::LogicalPosition::new(x, y),
        ));
    }

    #[cfg(target_os = "windows")]
    {
        let cursor = match windows_cursor() {
            Some(c) => c,
            None => { let _ = window.center(); return; }
        };
        // Windows: 光标与 monitor 都用物理像素;窗口尺寸是逻辑,需按 monitor scale 转物理
        let monitor = monitors.iter().find(|m| {
            let pos = m.position();
            let size = m.size();
            cursor.0 >= pos.x as f64
                && cursor.0 < (pos.x + size.width as i32) as f64
                && cursor.1 >= pos.y as f64
                && cursor.1 < (pos.y + size.height as i32) as f64
        }).unwrap_or(&monitors[0]);

        let s = monitor.scale_factor();
        let pw = width * s;
        let ph = height * s;
        let l = monitor.position().x as f64;
        let t = monitor.position().y as f64;
        let r = l + monitor.size().width as f64;
        let b = t + monitor.size().height as f64;
        let offset = CURSOR_OFFSET * s;

        let (x, y) = pick_pos(cursor.0, cursor.1, pw, ph, l, t, r, b, offset);
        let _ = window.set_position(tauri::Position::Physical(
            tauri::PhysicalPosition::new(x as i32, y as i32),
        ));
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        let _ = window.center();
    }
}

/// 选位:光标右下放得下就右下,否则反向(左上),最后 clamp 到屏内兜底。
#[allow(clippy::too_many_arguments)]
fn pick_pos(
    cx: f64, cy: f64, w: f64, h: f64,
    l: f64, t: f64, r: f64, b: f64,
    offset: f64,
) -> (f64, f64) {
    let mut x = cx + offset;
    let mut y = cy + offset;
    if x + w > r { x = cx - offset - w; }
    if y + h > b { y = cy - offset - h; }
    if x < l { x = l; }
    if y < t { y = t; }
    if x + w > r { x = r - w; }
    if y + h > b { y = b - h; }
    (x, y)
}

#[cfg(target_os = "macos")]
fn macos_cursor() -> Option<(f64, f64)> {
    use core_graphics::event::CGEvent;
    use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};
    let source = CGEventSource::new(CGEventSourceStateID::HIDSystemState).ok()?;
    let event = CGEvent::new(source).ok()?;
    let p = event.location();
    Some((p.x, p.y))
}

#[cfg(target_os = "windows")]
fn windows_cursor() -> Option<(f64, f64)> {
    use windows_sys::Win32::Foundation::POINT;
    use windows_sys::Win32::UI::WindowsAndMessaging::GetCursorPos;
    let mut p = POINT { x: 0, y: 0 };
    if unsafe { GetCursorPos(&mut p) } == 0 { return None; }
    Some((p.x as f64, p.y as f64))
}
