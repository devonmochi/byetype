# OCR 结果窗口钉住多开 实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 让被钉住的 OCR 结果窗原地保留，再次识别时新结果另开新窗，可同时存在任意多个钉住窗。

**Architecture:** 把预览窗从「单标签 `preview` + 全局 static 状态」的强单例，改造为「序号标签 `preview-{N}` + 活动槽 `ACTIVE_LABEL` + 每窗独立状态」的多实例模型。没钉住的窗作为活动槽随用随换；点钉住即「毕业」——活动槽腾空，该窗原地留底。前后端事件通信由全局广播改为按窗口标签定向（`emit_to` + 各窗自身 listen），避免多窗串台。

**Tech Stack:** Rust (Tauri v2)、React + TypeScript、`@tauri-apps/api`。

## Global Constraints

- 提交信息统一用中文（仓库约定，保证自动生成的 release notes 为中文）
- 不破坏现有「不钉住连续识别 = 复用同一窗替换」行为（回归基线）
- 沿用现有窗口定位逻辑（`position_near_cursor`，跟随光标）
- Tauri 窗口标签创建后不可改名；新标签需在 capabilities 中授权
- 本地开发运行：`npm run tauri dev`

---

## File Structure

- `src-tauri/capabilities/default.json` — 把窗口权限 `"preview"` 放宽为 `"preview*"`，覆盖所有序号标签
- `src-tauri/src/preview.rs` — 主战场：状态模型、序号标签、活动槽、`emit` 改 `emit_to`、按标签 blur/close/prewarm
- `src/views/preview/App.tsx` — 前端：按窗 listen、回执与命令调用带 label
- `src-tauri/src/task/mod.rs` — 调用点 `show`/`prewarm`/`close_if_exists` 签名不变，无需改动（仅核对）

---

## Task 1: 放宽窗口权限到 preview*

**Files:**
- Modify: `src-tauri/capabilities/default.json:4`

**Interfaces:**
- Consumes: 无
- Produces: 让标签形如 `preview-1`、`preview-2` 的窗口获得与原 `preview` 相同的 core window/webview/event 权限

这是整个改造的前置条件。Tauri v2 的 capability `windows` 字段按标签匹配，`"preview"` 是精确匹配，新序号标签不在其内将导致这些窗口拿不到 `core:window:*` 等权限、命令调用与事件失效。改为 glob `"preview*"` 即可覆盖 `preview-{N}`。

- [ ] **Step 1: 修改窗口权限匹配**

把第 4 行：

```json
  "windows": ["settings", "worker", "bubble*", "preview"],
```

改为：

```json
  "windows": ["settings", "worker", "bubble*", "preview*"],
```

- [ ] **Step 2: 提交**

```bash
git add src-tauri/capabilities/default.json
git commit -m "chore(preview): 放宽窗口权限匹配到 preview* 以支持多实例标签"
```

---

## Task 2: 后端状态模型改造（编译通过为准）

**Files:**
- Modify: `src-tauri/src/preview.rs:1-34`（imports + static 定义 + 命令签名）

**Interfaces:**
- Consumes: 无
- Produces:
  - `static WINDOW_SEQ: AtomicU64`
  - `static ACTIVE_LABEL: Mutex<Option<String>>`
  - `static PINNED_LABELS: Mutex<HashSet<String>>`
  - `static READY_LABELS: Mutex<HashMap<String, bool>>`
  - `static CREATED_AT: Mutex<HashMap<String, u64>>`
  - `fn next_label() -> String` 返回 `format!("preview-{}", n)`
  - 命令 `set_preview_pinned(label: String, pinned: bool)`
  - 命令 `close_preview_window(app: AppHandle, label: String)`

本任务只改「状态容器 + 命令签名 + 辅助函数」，让结构先就位；`show`/`prewarm`/blur 的逻辑改造留到后续任务。本步会临时破坏 `show`/`prewarm` 中对旧 static 的引用，故本任务的验收是「定义就位」，整体编译在 Task 3-5 完成后恢复。

> 说明：此项目无 Rust 单元测试覆盖该模块，验证以 `cargo check` 编译状态为准（行为测试在 Task 6 手动回归）。

- [ ] **Step 1: 替换 imports 与 static 定义**

把文件顶部第 1-15 行：

```rust
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
```

替换为：

```rust
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Listener, Manager, WebviewUrl, WebviewWindowBuilder};

/// 光标与预览窗口之间的边距(逻辑像素/物理像素,取决于平台 set_position 分支)。
const CURSOR_OFFSET: f64 = 12.0;
const BLUR_GRACE_MS: u128 = 800;

/// 自增窗口序号,生成永不复用的标签 preview-{N}。
static WINDOW_SEQ: AtomicU64 = AtomicU64::new(0);
/// 当前「活动槽」窗口标签:没钉住的临时窗,可被复用替换内容。None 表示槽空。
static ACTIVE_LABEL: Mutex<Option<String>> = Mutex::new(None);
/// 已钉住的窗口标签集合:这些窗失焦不自动关闭。
static PINNED_LABELS: Mutex<HashSet<String>> = Mutex::new(HashSet::new());
/// 各窗前端 React 是否已 mount 并注册 listener,按标签存。
static READY_LABELS: Mutex<HashMap<String, bool>> = Mutex::new(HashMap::new());
/// 各窗创建时间(epoch millis),按标签存,用于 blur 宽限期判断。
static CREATED_AT: Mutex<HashMap<String, u64>> = Mutex::new(HashMap::new());

/// 生成下一个永不复用的窗口标签。
fn next_label() -> String {
    let n = WINDOW_SEQ.fetch_add(1, Ordering::Relaxed);
    format!("preview-{}", n)
}

/// 标签对应窗是否已钉住。
fn is_pinned(label: &str) -> bool {
    PINNED_LABELS.lock().unwrap().contains(label)
}
```

- [ ] **Step 2: 改写 set_preview_pinned 命令**

把第 24-27 行：

```rust
#[tauri::command]
pub fn set_preview_pinned(pinned: bool) {
    PINNED.store(pinned, Ordering::Relaxed);
}
```

替换为：

```rust
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
```

- [ ] **Step 3: 改写 close_preview_window 命令**

把第 29-34 行：

```rust
#[tauri::command]
pub fn close_preview_window(app: AppHandle) {
    if let Some(window) = app.get_webview_window("preview") {
        let _ = window.close();
    }
}
```

替换为：

```rust
#[tauri::command]
pub fn close_preview_window(app: AppHandle, label: String) {
    if let Some(window) = app.get_webview_window(&label) {
        let _ = window.close();
    }
}
```

- [ ] **Step 4: 编译核对（预期此时仍有错误）**

Run: `cd src-tauri && cargo check`
Expected: 报错集中在 `show`/`prewarm`/blur 中对已删除的 `PINNED`/`PREVIEW_READY`/`BLUR_HANDLER_REGISTERED` 的引用。这是预期的，下一任务修复。不提交。

---

## Task 3: 改造 show() 为活动槽 + 定向通信

**Files:**
- Modify: `src-tauri/src/preview.rs:36-164`（`show` 函数整体）

**Interfaces:**
- Consumes: `next_label()`、`ACTIVE_LABEL`、`READY_LABELS`、`CREATED_AT`、`is_pinned`、`PINNED_LABELS`（Task 2）
- Produces: `pub fn show(app: &AppHandle, text: &str) -> Result<(), String>`（签名不变，调用点无需改）

关键点：所有 `emit`/`once` 改为针对具体 label 的定向通信；窗口选择由 `ACTIVE_LABEL` 决定，而非写死 `"preview"`。前端回执事件名改为带标签后缀（`preview-ready-{label}`、`preview-text-applied-{label}`），避免多窗广播串台。文本下发用 `emit_to(&label, "preview-text", ...)`。

- [ ] **Step 1: 用新实现替换整个 show 函数**

把第 36-164 行的 `pub fn show(...) { ... }` 整体替换为：

```rust
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
```

> 注意：`window_for_applied.get_webview_window(...)` 中 `WebviewWindow` 实现了 `Manager`，可直接取窗口；若编译器提示 trait 未引入，确认顶部 `use tauri::Manager` 已在（Task 2 保留了它）。

- [ ] **Step 2: 暂不编译（等 Task 4 的 register_window_events 与 now_ms）**

`now_ms()` 在文件中已存在（第 17-22 行，保留不动）。`register_window_events` 在 Task 4 定义。本步不单独编译。

---

## Task 4: 每窗事件处理 register_window_events

**Files:**
- Modify: `src-tauri/src/preview.rs`（在 `show` 之后、`prewarm` 之前新增函数）

**Interfaces:**
- Consumes: `is_pinned`、`CREATED_AT`、`READY_LABELS`、`PINNED_LABELS`、`ACTIVE_LABEL`、`BLUR_GRACE_MS`、`now_ms`
- Produces: `fn register_window_events(app: &AppHandle, window: &tauri::WebviewWindow, label: String)`

每个窗口独立注册 blur/destroy。blur 时按自己的标签判断是否钉住、是否过宽限期；destroy 时清理本标签在各容器里的项，并在自己是活动槽时腾空。

- [ ] **Step 1: 新增 register_window_events 函数**

在 `show` 函数闭合的 `}` 之后插入：

```rust
/// 为单个预览窗注册 blur(失焦关闭)与 destroy(状态清理)处理。
/// 每个窗口各自注册一次,按自己的标签判断,互不干扰。
fn register_window_events(app: &AppHandle, window: &tauri::WebviewWindow, label: String) {
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
            let mut active = ACTIVE_LABEL.lock().unwrap();
            if active.as_deref() == Some(label.as_str()) {
                *active = None;
            }
        }
        _ => {}
    });
}
```

- [ ] **Step 2: 编译核对**

Run: `cd src-tauri && cargo check`
Expected: `show` 与 `register_window_events` 相关错误消失；可能仍剩 `prewarm`/`close_if_exists` 对旧逻辑的引用错误（Task 5 修复）。

---

## Task 5: 改造 prewarm 与 close_if_exists

**Files:**
- Modify: `src-tauri/src/preview.rs:171-221`（`prewarm` 与 `close_if_exists`）

**Interfaces:**
- Consumes: `next_label`、`ACTIVE_LABEL`、`READY_LABELS`、`next_label`
- Produces: `pub fn prewarm(app: &AppHandle)`、`pub fn close_if_exists(app: &AppHandle)`（签名不变）

预热只服务「下一个活动窗」：活动槽已有则跳过；否则建一个序号标签隐藏窗并写入活动槽。`close_if_exists` 只关「当前活动槽」那个窗（失败路径清理预热残留），不动钉住窗。

- [ ] **Step 1: 替换 prewarm 函数**

把第 171-214 行的 `pub fn prewarm(...) { ... }` 整体替换为：

```rust
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
```

- [ ] **Step 2: 替换 close_if_exists 函数**

把第 216-221 行：

```rust
/// 供失败路径调用:若存在 preview 窗口(可能是预热残留)则关闭。
pub fn close_if_exists(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("preview") {
        let _ = window.close();
    }
}
```

替换为：

```rust
/// 供失败路径调用:关闭当前活动槽里的窗(可能是预热残留)。不影响已钉住窗。
pub fn close_if_exists(app: &AppHandle) {
    let label = ACTIVE_LABEL.lock().unwrap().clone();
    if let Some(label) = label {
        if let Some(window) = app.get_webview_window(&label) {
            let _ = window.close();
        }
    }
}
```

- [ ] **Step 3: 编译核对（预期通过）**

Run: `cd src-tauri && cargo check`
Expected: 编译通过（warning 可接受）。后端改造完成。

- [ ] **Step 4: 提交后端**

```bash
git add src-tauri/src/preview.rs
git commit -m "feat(preview): 后端改造为多实例加活动槽模型，事件按标签定向"
```

---

## Task 6: 前端按窗通信与带标签命令

**Files:**
- Modify: `src/views/preview/App.tsx:1-138`

**Interfaces:**
- Consumes: 后端事件 `preview-text`（定向到本窗）、后端命令 `set_preview_pinned(label, pinned)`、`close_preview_window(label)`
- Produces: 前端发出 `preview-ready-{label}`、`preview-text-applied-{label}` 回执

前端取当前窗口 label，监听只收本窗的 `preview-text`，回执事件名带 label 后缀，钉住/关闭命令带上 label。

- [ ] **Step 1: 引入当前窗口 label 并改造文本监听 effect**

把第 100-112 行的文本监听 effect：

```tsx
  useEffect(() => {
    let unlisten: (() => void) | null = null
    listen<string>('preview-text', (event) => {
      // flushSync 强制 React 在本行返回前把 DOM commit 完成,
      // 保证后端收到 applied 回执 → window.show() 时,屏幕已是新文本而非旧 state。
      flushSync(() => setText(event.payload))
      emit('preview-text-applied', {})
    }).then((fn) => {
      unlisten = fn
      emit('preview-ready', {})
    })
    return () => { unlisten?.() }
  }, [])
```

替换为：

```tsx
  useEffect(() => {
    const win = getCurrentWindow()
    const label = win.label
    let unlisten: (() => void) | null = null
    // 只监听定向到本窗的 preview-text;回执带 label 后缀,避免多窗广播串台。
    win.listen<string>('preview-text', (event) => {
      // flushSync 强制 React 在本行返回前把 DOM commit 完成,
      // 保证后端收到 applied 回执 → window.show() 时,屏幕已是新文本而非旧 state。
      flushSync(() => setText(event.payload))
      emit(`preview-text-applied-${label}`, {})
    }).then((fn) => {
      unlisten = fn
      emit(`preview-ready-${label}`, {})
    })
    return () => { unlisten?.() }
  }, [])
```

- [ ] **Step 2: 改造 handlePin 带 label**

把第 130-134 行：

```tsx
  const handlePin = async () => {
    const next = !pinned
    setPinned(next)
    await invoke('set_preview_pinned', { pinned: next })
  }
```

替换为：

```tsx
  const handlePin = async () => {
    const next = !pinned
    setPinned(next)
    await invoke('set_preview_pinned', { label: getCurrentWindow().label, pinned: next })
  }
```

- [ ] **Step 3: 改造 handleClose 带 label**

把第 136-138 行：

```tsx
  const handleClose = () => {
    invoke('close_preview_window')
  }
```

替换为：

```tsx
  const handleClose = () => {
    invoke('close_preview_window', { label: getCurrentWindow().label })
  }
```

- [ ] **Step 4: 清理未使用的 import**

第 4 行原为 `import { emit, listen } from '@tauri-apps/api/event'`。改造后 `listen`（全局）已不再使用，改为：

```tsx
import { emit } from '@tauri-apps/api/event'
```

（`getCurrentWindow` 已在第 5 行 import，无需新增。）

- [ ] **Step 5: 前端类型检查**

Run: `npm run build`（或项目的 tsc 检查命令）
Expected: 通过，无 TS 报错（确认 `listen` 移除后无残留引用）。

- [ ] **Step 6: 提交前端**

```bash
git add src/views/preview/App.tsx
git commit -m "feat(preview): 前端事件按窗定向，钉住与关闭命令带窗口标签"
```

---

## Task 7: 手动回归验证

**Files:**
- 无（运行验证）

**Interfaces:**
- Consumes: 全部前述任务
- Produces: 验证报告

本项目为桌面 GUI（Tauri），核心行为靠手动验证。

- [ ] **Step 1: 启动开发模式**

Run: `npm run tauri dev`
Expected: 应用正常启动，无控制台权限错误（验证 Task 1 的 `preview*` 生效）。

- [ ] **Step 2: 回归基线 — 不钉住连续识别**

操作：连续做两次 OCR（都不点钉住）。
Expected: 始终复用同一窗口、内容被新结果替换，不堆叠新窗（行为与改造前一致）。

- [ ] **Step 3: 钉住后再识别 — 多开**

操作：识别一次 → 点钉住（图钉竖直）→ 再识别一次。
Expected: 第一个窗原地保留、内容不变；第二个结果出现在新窗口。

- [ ] **Step 4: 串台验证（关键）**

操作：识别 A → 钉住 A → 识别 B。
Expected: A 窗内容保持为 A，没有被 B 冲掉（验证 `emit_to` 定向通信生效）。

- [ ] **Step 5: 多钉住**

操作：识别 A → 钉住 → 识别 B → 钉住 → 识别 C。
Expected: A、B 两个钉住窗都在，C 是新的活动窗。

- [ ] **Step 6: 失焦关闭 & 清理**

操作：识别但不钉住 → 点击别处使其失焦（超过 0.8 秒）。再：钉住一个窗 → 点击别处失焦。
Expected: 未钉住窗失焦后自动关闭；钉住窗失焦后保留。关闭活动窗后再次识别能正常新建。

- [ ] **Step 7: 取消钉住**

操作：钉住一个窗 → 再点一次取消钉住 → 点击别处失焦。
Expected: 取消后该窗按未钉住规则，失焦自动关闭。

---

## Self-Review 记录

- **Spec coverage:** 架构（Task 2-5）、活动槽与钉住毕业（Task 2 命令 + Task 3 show）、事件串台解法（Task 3/6 的 emit_to + 带 label 回执）、失焦关闭与剪贴板（Task 4 + 现有 handleBlur 不变）、预热（Task 5）、改动文件清单（Task 1/2/6 覆盖；task/mod.rs 经核对调用签名不变无需改）、测试要点（Task 7 全覆盖）。新增 spec 未显式提及但实测必需的 capabilities 权限项（Task 1）。
- **Placeholder scan:** 无 TBD/TODO；每个代码步给出完整替换代码。
- **Type consistency:** 命令签名 `set_preview_pinned(label, pinned)`、`close_preview_window(label)` 前后端一致；事件名 `preview-ready-{label}` / `preview-text-applied-{label}` / `preview-text`（定向）前后端一致；`ACTIVE_LABEL`/`PINNED_LABELS`/`READY_LABELS`/`CREATED_AT` 在各任务引用一致。
