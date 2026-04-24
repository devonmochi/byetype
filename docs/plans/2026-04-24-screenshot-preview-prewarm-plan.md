# 截图预览窗口预热优化 实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** macOS 下截图 OCR 流程中,将「AI 返回 → 预览框出现」的空白延迟从 ~800ms 降至 <100ms,且不增加常驻内存。

**Architecture:** 在截图选区完成、AI 调用即将开始时,并行预热一个隐藏的预览 webview。AI 调用 2-5s 足以掩盖 webview 创建的 500-1000ms,AI 返回时复用已创建好的窗口,直接 emit 文本并显示。失败时关闭隐藏窗口,零常驻内存。

**Tech Stack:** Rust / Tauri v2 (`WebviewWindowBuilder`, `app.run_on_main_thread`, `app.get_webview_window`)

---

## 背景参考

完整方案背景、权衡、边界分析见:`docs/plans/2026-04-24-screenshot-preview-prewarm-design.md`

## 文件结构

- **修改** `src-tauri/src/preview.rs` — 新增 `prewarm()` 与 `close_if_exists()`,重构 `show()` 支持窗口复用
- **修改** `src-tauri/src/task/mod.rs` — `run_extract_pipeline` 调用 `prewarm`,`finish_extract_pipeline` 失败分支调用 `close_if_exists`

不改动前端(`preview.html` / `src/views/preview/App.tsx`),现有 `preview-ready` + `preview-text` 握手已支持窗口先在、文本后到的时序。

## 项目约定

- 所有命令从 worktree 根目录执行:`/Users/devonmochi/PycharmProjects/pythonProject1/日常工具/byetype/.claude/worktrees/screenshot-preview-prewarm`
- 本项目无自动化测试套件,采用「编译通过 + 手动验证」作为 Task 完成标准
- 开发运行命令:`npm run tauri dev`(项目 `CLAUDE.md` 已约定)
- commit 信息使用中文(项目 `CLAUDE.md` + auto-memory 已约定)

---

## Task 1: 新增 `preview::prewarm()` — 预热隐藏窗口

**Files:**
- Modify: `src-tauri/src/preview.rs`

**说明:** `prewarm` 职责是幂等创建一个隐藏的预览窗口,让 React bundle 后台加载。Tauri 的 `WebviewWindowBuilder::build()` 必须在主线程执行,因此通过 `app.run_on_main_thread()` 分派。不 emit 文本、不绑定 blur、不 show —— 这些都留给后续 `show()` 处理。

- [ ] **Step 1.1: 在 `src-tauri/src/preview.rs` 末尾追加 `prewarm` 函数**

在文件末尾(`show()` 函数之后)追加:

```rust
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
    let _ = app.run_on_main_thread(move || {
        // 主线程上再次检查,防止两次 prewarm 之间的竞态
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
    });
}
```

- [ ] **Step 1.2: 验证 Rust 侧编译通过**

在 worktree 根目录执行:

```bash
cd src-tauri && cargo check
```

预期:`Finished` 输出,无 error。若有 warning 关于未使用的 `prewarm`,可忽略(下个 Task 会调用)。

- [ ] **Step 1.3: Commit**

```bash
cd ..
git add src-tauri/src/preview.rs
git commit -m "feat: 新增 preview::prewarm 幂等预热隐藏预览窗口"
```

---

## Task 2: 新增 `preview::close_if_exists()` — 供失败路径清理

**Files:**
- Modify: `src-tauri/src/preview.rs`

**说明:** AI 失败时调用,关闭可能残留的预热隐藏窗口。与现有 `close_preview_window` 命令行为一致,但不是 `#[tauri::command]`(供 Rust 内部调用)。保留独立函数的意义是语义清晰:`close_preview_window` 是前端触发的主动关闭,`close_if_exists` 是后端失败清理。

- [ ] **Step 2.1: 在 `preview.rs` 追加 `close_if_exists` 函数**

追加到 `prewarm` 函数之后:

```rust
/// 供失败路径调用:若存在 preview 窗口(可能是预热残留)则关闭。
pub fn close_if_exists(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("preview") {
        let _ = window.close();
    }
}
```

- [ ] **Step 2.2: 验证编译**

```bash
cd src-tauri && cargo check
```

预期:无 error。

- [ ] **Step 2.3: Commit**

```bash
cd ..
git add src-tauri/src/preview.rs
git commit -m "feat: 新增 preview::close_if_exists 供失败路径清理隐藏窗口"
```

---

## Task 3: 重构 `preview::show()` — 支持复用已预热窗口

**Files:**
- Modify: `src-tauri/src/preview.rs`(`show` 函数,第 28-84 行)

**说明:** 当前 `show()` 每次都关闭旧窗口后新建。改为:如果窗口已由 `prewarm` 创建,复用它——跳过 build,通过 `set_size` 调整尺寸。

**时序策略(双路径)**:前端 `App.tsx` 会在 `listen('preview-text')` 注册后 `emit('preview-ready')` 一次(见 `src/views/preview/App.tsx:99-108`)。为确保两种场景都能正确显示,`show()` 同时做两件事:

1. 立即 emit + show(快路径):窗口若是预热且 React 已 mount,前端 listen 已就位,此 emit 立刻生效,瞬时显示
2. 注册 `once('preview-ready')`(慢路径):窗口若是新建(回退),此时前端 listen 还没注册,立即 emit 会被丢弃;等 React mount 后 emit `preview-ready`,触发 once 回调再 emit + show

两路径并存无害:快路径触发时慢路径的 once 可能永远等不到(ready 已经 emit 过),但不 leak 资源;若快路径没生效,慢路径兜底。

- [ ] **Step 3.1: 用下列新实现替换 `preview.rs` 第 28-84 行的整个 `show` 函数**

```rust
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

    // 注册 ready 握手:若前端尚未 emit preview-ready,等到就 emit 文本并 show
    let text_clone = text.to_string();
    let window_for_ready = window.clone();
    window.once("preview-ready", move |_| {
        let _ = window_for_ready.emit("preview-text", &text_clone);
        let _ = window_for_ready.show();
    });

    // 同时立即 emit 一次:若窗口是预热的且 React 已 mount,此次 emit 会立刻生效,实现「瞬时显示」
    let _ = window.emit("preview-text", text);
    let _ = window.show();

    // 记录创建时间用于 blur 宽限期
    CREATED_AT.store(now_ms(), Ordering::Relaxed);

    // blur 关闭事件(不固定 && 过宽限期)
    let app_handle = app.clone();
    window.on_window_event(move |event| {
        if let tauri::WindowEvent::Focused(false) = event {
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
    });

    Ok(())
}
```

**关键变化**:
1. 不再无条件关闭旧窗口;改为若存在则复用
2. 尺寸计算提前到分支之前共享
3. 新增「立即 emit + show」作为快路径(针对预热窗口场景)
4. 保留 `once('preview-ready')` 作为慢路径(针对新建窗口场景)

- [ ] **Step 3.2: 验证编译**

```bash
cd src-tauri && cargo check
```

预期:无 error。

- [ ] **Step 3.3: Commit**

```bash
cd ..
git add src-tauri/src/preview.rs
git commit -m "refactor: preview::show 支持复用已预热窗口,双路径保证显示时机"
```

---

## Task 4: 在 OCR 流程中触发预热

**Files:**
- Modify: `src-tauri/src/task/mod.rs`(`run_extract_pipeline` 函数,第 489-585 行)

**说明:** 在 `run_extract_pipeline` 拿到 `image_base64`、进入 AI 调用前触发 `prewarm`。位置选在「bubble 更新为 extracting」之后、「build_client」之前 —— 此时 AI 还没开始,预热最有价值。

- [ ] **Step 4.1: 在 `task/mod.rs` 中 `bubble::update(app, task_id, "extracting")` 之后加一行 prewarm**

定位到第 500-503 行附近的代码:

```rust
    // Show bubble with extracting status
    let _ = crate::bubble::show(app, task_id);
    let _ = crate::bubble::update(app, task_id, "extracting");
```

在这三行之后、`let (config, prompts_dir, token) = {` 之前插入:

```rust
    // 预热预览窗口,与 AI 调用并行,掩盖 webview 冷启动开销
    crate::preview::prewarm(app);
```

- [ ] **Step 4.2: 在 `finish_extract_pipeline` 失败分支调用 `close_if_exists`**

定位到 `finish_extract_pipeline` 函数(第 587 行开始)。在 `bubble::hide` 之后、`history` 保存之前加入清理逻辑。

找到这段:

```rust
    // Update bubble
    let bubble_delay = if status == "completed" { 1500 } else { 3000 };
    let _ = crate::bubble::update(app, task_id, status);
    let _ = crate::bubble::hide(app, task_id, bubble_delay);
```

在这段之后、`// Save history` 之前插入:

```rust
    // 失败路径:关闭可能残留的预热隐藏窗口(成功路径由 show() 自己复用,不清理)
    if status != "completed" {
        crate::preview::close_if_exists(app);
    }
```

- [ ] **Step 4.3: 验证编译**

```bash
cd src-tauri && cargo check
```

预期:无 error 无 warning。

- [ ] **Step 4.4: Commit**

```bash
cd ..
git add src-tauri/src/task/mod.rs
git commit -m "feat: 截图 OCR 流程中预热预览窗口,失败时清理残留"
```

---

## Task 5: 手动验证

**说明:** 本项目无自动化测试套件,通过启动 dev 模式手动验证。

- [ ] **Step 5.1: 启动 dev 模式**

```bash
npm run tauri dev
```

等待编译完成、应用启动(几十秒)。

- [ ] **Step 5.2: 效果验证(主目标)**

1. 按截图快捷键(默认 F6)
2. 选一个区域截图
3. 观察:AI 返回文本的时刻 → 预览窗口显示的时刻
4. 预期:这段间隔应当从原来的 ~800ms 降到近乎瞬时(<100ms)

若改进不明显,检查控制台是否有 `[preview] prewarm failed` 日志。

- [ ] **Step 5.3: 退化验证**

1. 在 `task/mod.rs` 里临时注释掉 `crate::preview::prewarm(app);` 那一行
2. 重新 `npm run tauri dev`,再次截图
3. 预期:功能正常,但弹窗延迟恢复到 ~800ms(证明 prewarm 是生效的)
4. **验证后取消注释,保留改动**

- [ ] **Step 5.4: 失败清理验证**

1. 临时改 AI 配置为错误的 API key(或断网)
2. 截图触发 OCR → AI 失败
3. 预期:不出现空白预览窗口;再次截图仍能正常预热并显示
4. **恢复正确配置**

- [ ] **Step 5.5: 连续截图验证**

1. 快速连续截图两次(首次 AI 还在思考时触发第二次)
2. 预期:根据 `max_parallel` 配置,要么第二次被拒绝,要么两次都成功——无窗口错乱、无 panic

- [ ] **Step 5.6: 如所有验证通过,无需额外 commit。若发现 bug,修复并 commit**

---

## Task 6: 交付前收尾

- [ ] **Step 6.1: 确认所有改动均已 commit**

```bash
git status
```

预期:`working tree clean`。

- [ ] **Step 6.2: 查看提交历史**

```bash
git log --oneline master..HEAD
```

预期:看到 4 个 commit(Task 1、2、3、4 各一个)。

- [ ] **Step 6.3: 报告完成**

向用户报告:
- worktree 路径
- 提交列表
- 手动验证结论
- 等待用户决定:合入 master / 发 PR / 其他
