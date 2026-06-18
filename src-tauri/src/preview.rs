use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{LazyLock, Mutex};
use tauri::{AppHandle, Emitter, Listener, Manager, WebviewUrl, WebviewWindowBuilder};

/// 光标与预览窗口之间的边距(逻辑像素/物理像素,取决于平台 set_position 分支)。
const CURSOR_OFFSET: f64 = 12.0;
const BLUR_GRACE_MS: u128 = 800;

/// 自增窗口序号,生成永不复用的标签 preview-{N}。
static WINDOW_SEQ: AtomicU64 = AtomicU64::new(0);
/// 当前「活动槽」窗口标签:没钉住的临时窗,可被复用替换内容。None 表示槽空。
static ACTIVE_LABEL: Mutex<Option<String>> = Mutex::new(None);
/// 已钉住的窗口标签集合:这些窗失焦不自动关闭。
static PINNED_LABELS: LazyLock<Mutex<HashSet<String>>> =
    LazyLock::new(|| Mutex::new(HashSet::new()));
/// 各窗前端 React 是否已 mount 并注册 listener,按标签存。
static READY_LABELS: LazyLock<Mutex<HashMap<String, bool>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));
/// 各窗创建时间(epoch millis),按标签存,用于 blur 宽限期判断。
static CREATED_AT: LazyLock<Mutex<HashMap<String, u64>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));
/// 已注册过 blur/destroy 监听的窗口标签集合,保证每个窗口只注册一次
/// (复用活动窗时 show 会重复调用,必须靠此守卫避免监听器叠加)。
static REGISTERED_LABELS: LazyLock<Mutex<HashSet<String>>> =
    LazyLock::new(|| Mutex::new(HashSet::new()));

/// 生成下一个永不复用的窗口标签。
fn next_label() -> String {
    let n = WINDOW_SEQ.fetch_add(1, Ordering::Relaxed);
    format!("preview-{}", n)
}

/// 标签对应窗是否已钉住。
fn is_pinned(label: &str) -> bool {
    PINNED_LABELS.lock().unwrap().contains(label)
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

#[tauri::command]
pub fn set_preview_pinned(label: String, pinned: bool) {
    if pinned {
        PINNED_LABELS.lock().unwrap().insert(label.clone());
        // 钉住即「毕业」:若它正是活动槽,腾空活动槽,下次识别新建活动窗。
        let mut active = ACTIVE_LABEL.lock().unwrap();
        if active.as_deref() == Some(label.as_str()) {
            *active = None;
        }
    } else {
        PINNED_LABELS.lock().unwrap().remove(&label);
    }
}

#[tauri::command]
pub fn close_preview_window(app: AppHandle, label: String) {
    if let Some(window) = app.get_webview_window(&label) {
        let _ = window.close();
    }
}

pub fn show(app: &AppHandle, text: &str) -> Result<(), String> {
    // 按文本计算尺寸
    let line_count = text.lines().count().max(3).min(20);
    let max_line_len = text.lines().map(|l| l.len()).max().unwrap_or(40);
    let width = (max_line_len as f64 * 8.0 + 80.0).clamp(320.0, 600.0);
    let height = (line_count as f64 * 22.0 + 140.0).clamp(180.0, 460.0);

    let shown = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));

    // 决定目标窗口:活动槽有窗则复用,否则新建并写入活动槽。
    let active_label = ACTIVE_LABEL.lock().unwrap().clone();
    let (label, window) = match active_label.and_then(|l| {
        app.get_webview_window(&l).map(|w| (l, w))
    }) {
        Some((label, existing)) => {
            // 复用活动窗:按文本调整尺寸
            let _ = existing.set_size(tauri::LogicalSize::new(width, height));

            let ready = READY_LABELS
                .lock()
                .unwrap()
                .get(&label)
                .copied()
                .unwrap_or(false);
            if !ready {
                // 半热复用:窗口已存在但前端还没发 ready,等 ready 后补发文本。
                let text_for_ready = text.to_string();
                let window_for_ready = existing.clone();
                let label_for_ready = label.clone();
                let ready_event = format!("preview-ready-{}", label);
                existing.once(ready_event, move |_| {
                    READY_LABELS
                        .lock()
                        .unwrap()
                        .insert(label_for_ready.clone(), true);
                    let _ = window_for_ready.emit_to(
                        &label_for_ready,
                        "preview-text",
                        &text_for_ready,
                    );
                });
            }
            (label, existing)
        }
        None => {
            // 新建活动窗
            let label = next_label();
            READY_LABELS.lock().unwrap().insert(label.clone(), false);
            let built = WebviewWindowBuilder::new(
                app,
                &label,
                WebviewUrl::App("preview.html".into()),
            )
            .title("ByeType Preview")
            .inner_size(width, height)
            .resizable(true)
            .decorations(false)
            .always_on_top(true)
            .visible(false)
            .build()
            .map_err(|e| format!("Create preview window failed: {}", e))?;
            *ACTIVE_LABEL.lock().unwrap() = Some(label.clone());

            let text_for_ready = text.to_string();
            let window_for_ready = built.clone();
            let label_for_ready = label.clone();
            let ready_event = format!("preview-ready-{}", label);
            built.once(ready_event, move |_| {
                READY_LABELS
                    .lock()
                    .unwrap()
                    .insert(label_for_ready.clone(), true);
                let _ = window_for_ready.emit_to(
                    &label_for_ready,
                    "preview-text",
                    &text_for_ready,
                );
            });
            (label, built)
        }
    };

    // 定位到光标右下——必须在 show() 之前完成
    position_near_cursor(&window, width, height);

    let window_for_applied = window.clone();
    let label_for_applied = label.clone();
    let shown_for_applied = shown.clone();
    let applied_event = format!("preview-text-applied-{}", label);
    window.once(applied_event, move |_| {
        if shown_for_applied
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
        {
            if let Some(w) = window_for_applied.get_webview_window(&label_for_applied) {
                let _ = w.show();
            }
        }
    });

    // 立即定向 emit 一次:预热且前端已 mount 时会被立刻接收;冷启动时丢失,由 ready 路径兜底。
    let _ = window.emit_to(&label, "preview-text", text);

    // 记录创建时间用于 blur 宽限期
    CREATED_AT
        .lock()
        .unwrap()
        .insert(label.clone(), now_ms());

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

    // 每个窗口注册自己的 blur/destroy 处理(按标签判断,互不影响)
    register_window_events(app, &window, label.clone());

    Ok(())
}

/// 为单个预览窗注册 blur(失焦关闭)与 destroy(状态清理)处理。
/// 每个窗口只注册一次:复用活动窗时 show 会重复调用,靠 REGISTERED_LABELS 守卫拦截,
/// 避免监听器叠加导致同一 blur 被多次处理、窗口被提前关闭或状态错乱。
fn register_window_events(app: &AppHandle, window: &tauri::WebviewWindow, label: String) {
    // 已注册过则跳过(复用路径)
    if !REGISTERED_LABELS.lock().unwrap().insert(label.clone()) {
        return;
    }
    let app_handle = app.clone();
    window.on_window_event(move |event| match event {
        tauri::WindowEvent::Focused(false) => {
            if is_pinned(&label) {
                return;
            }
            let created = CREATED_AT
                .lock()
                .unwrap()
                .get(&label)
                .copied()
                .unwrap_or(0);
            let age = now_ms().saturating_sub(created);
            if (age as u128) < BLUR_GRACE_MS {
                return;
            }
            if let Some(w) = app_handle.get_webview_window(&label) {
                let _ = w.close();
            }
        }
        tauri::WindowEvent::Destroyed => {
            READY_LABELS.lock().unwrap().remove(&label);
            CREATED_AT.lock().unwrap().remove(&label);
            PINNED_LABELS.lock().unwrap().remove(&label);
            REGISTERED_LABELS.lock().unwrap().remove(&label);
            let mut active = ACTIVE_LABEL.lock().unwrap();
            if active.as_deref() == Some(label.as_str()) {
                *active = None;
            }
        }
        _ => {}
    });
}

/// 预热:提前创建一个隐藏的预览窗口,让 React bundle 后台加载。
///
/// 幂等 —— 若 preview 窗口已存在则直接返回。调用发生在 AI 调用开始时,
/// 利用 AI 等待时间掩盖 webview 冷启动开销。失败只打 log,不中断主流程
/// (后续 show() 会走创建分支,退化到旧行为)。
pub fn prewarm(app: &AppHandle) {
    // 活动槽已有窗则跳过(幂等)
    if ACTIVE_LABEL.lock().unwrap().is_some() {
        return;
    }
    let app_cloned = app.clone();
    if let Err(e) = app.run_on_main_thread(move || {
        // 主线程上再次检查,防止调度延迟期间被重复派发
        if ACTIVE_LABEL.lock().unwrap().is_some() {
            return;
        }
        let label = next_label();
        READY_LABELS.lock().unwrap().insert(label.clone(), false);
        let result = WebviewWindowBuilder::new(
            &app_cloned,
            &label,
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
                *ACTIVE_LABEL.lock().unwrap() = Some(label.clone());
                // 预热窗的 React mount 完成会 emit preview-ready-{label},在此接住记录,
                // 否则 show() 时该信号无人接收,只能靠 200ms 兜底显示空白窗。
                let label_for_ready = label.clone();
                let ready_event = format!("preview-ready-{}", label);
                window.once(ready_event, move |_| {
                    READY_LABELS
                        .lock()
                        .unwrap()
                        .insert(label_for_ready.clone(), true);
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

/// 供失败路径调用:关闭当前活动槽里的窗(可能是预热残留)。不影响已钉住窗。
pub fn close_if_exists(app: &AppHandle) {
    let label = ACTIVE_LABEL.lock().unwrap().clone();
    if let Some(label) = label {
        if let Some(window) = app.get_webview_window(&label) {
            let _ = window.close();
        }
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
