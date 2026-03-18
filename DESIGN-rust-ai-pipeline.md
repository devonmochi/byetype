# ByeType 重构设计文档：AI 管道迁移至 Rust

## 目标

将 AI 调用管道从 JS Worker 窗口完全迁移到 Rust 端，消除 1x1 Worker 窗口，解决启动闪烁和焦点抢夺问题。

## 约束

- Settings 窗口保留现有 React 实现不变
- Bubble 窗口保留现有 React 渲染，复用已有 `bubble::show/update/hide` 接口
- 支持 Gemini 3 系列 + Qwen3-Omni-Flash + OpenAI 兼容接口
- 移除 Gemini 2.5 系列模型
- 使用 reqwest（已有 0.13.2）直连 HTTP API，不引入额外 SDK

---

## 整体架构

### 现有架构

```
快捷键 -> Rust 录音 -> emit("recording-complete") -> JS Worker
  -> JS TaskManager -> GeminiProvider / QwenOmniProvider -> 转录
  -> JS optimizeText -> OpenAI 兼容接口 -> 优化
  -> invoke("paste_text") -> Rust 剪贴板粘贴
  -> invoke("show_bubble/update_bubble/hide_bubble") -> Bubble 窗口更新
```

### 新架构

```
快捷键 -> Rust 录音 -> Rust TaskManager.process(audio)
  -> reqwest -> Gemini 3 / Qwen3 API -> 转录
  -> reqwest -> OpenAI 兼容 API -> 优化（可选）
  -> Rust 剪贴板粘贴
  -> Rust 直接调用 bubble::show/update/hide -> Bubble 窗口更新
```

---

## Rust 新增模块

### 目录结构

```
src-tauri/src/
  ai/
    mod.rs              -- AI 模块入口，统一 transcribe / optimize 接口
    gemini.rs           -- Gemini 3 REST API 客户端
    qwen.rs             -- Qwen3-Omni OpenAI 兼容客户端
    openai_compat.rs    -- 通用 OpenAI 兼容客户端（用于文本优化）
    types.rs            -- 请求/响应结构体定义
  task/
    mod.rs              -- TaskManager 状态机
    history.rs          -- 历史记录存储（磁盘模式）
```

### Gemini 3 客户端 (`ai/gemini.rs`)

API 端点：`https://generativelanguage.googleapis.com/v1beta/models/{model}:generateContent`

请求结构：
```json
{
  "system_instruction": { "parts": [{ "text": "系统提示词" }] },
  "contents": [{
    "parts": [
      { "inline_data": { "mime_type": "audio/wav", "data": "base64..." } },
      { "text": "用户提示词" }
    ]
  }],
  "generationConfig": {
    "thinking": { "thinkingLevel": "LOW" }
  }
}
```

注意：Gemini 3 系列使用 `thinkingLevel`（MINIMAL/LOW/MEDIUM/HIGH），不再使用 2.5 的 `thinkingBudget` 数字格式。

支持模型：
- `gemini-3-flash-preview`
- `gemini-3.1-flash-lite-preview`

### Qwen3 客户端 (`ai/qwen.rs`)

API 端点：`https://dashscope.aliyuncs.com/compatible-mode/v1/chat/completions`

请求结构（OpenAI 兼容格式，非流式）：
```json
{
  "model": "qwen3-omni-flash",
  "messages": [{
    "role": "user",
    "content": [
      { "type": "input_audio", "input_audio": { "data": "data:audio/wav;base64,...", "format": "wav" } },
      { "type": "text", "text": "系统提示词" }
    ]
  }],
  "modalities": ["text"]
}
```

注意：初期使用非流式请求简化实现。当前 JS 端虽使用 `stream: true`，但对语音输入场景（通常几秒音频）影响不大。后续可按需加回流式支持。

### 文本优化客户端 (`ai/openai_compat.rs`)

通用 OpenAI 兼容客户端，支持自定义 `base_url`。保持与现有 JS 实现一致的消息格式：
```json
{
  "model": "{model}",
  "messages": [{
    "role": "user",
    "content": "<voice-input>\n{text}\n</voice-input>\n\n{systemPrompt}"
  }]
}
```

复用于 Gemini 做优化的场景（通过 `generativelanguage.googleapis.com/v1beta/openai/` 兼容端点）。

### 提示词加载

复用现有 `commands.rs` 中 `resolve_prompts_dir` 的逻辑：优先读用户目录 `app_data_dir/prompts/`，不存在则回退到 `resource_dir/prompts/`（开发模式回退到 `CARGO_MANIFEST_DIR/prompts`）。

### 代理支持

复用现有 `proxy.rs` 模块的代理 URL，给 `reqwest::Client` 设置 proxy：
```rust
let mut builder = reqwest::Client::builder();
if !proxy_url.is_empty() {
    builder = builder.proxy(reqwest::Proxy::all(proxy_url)?);
}
let client = builder.build()?;
```

### 重试与超时机制

对标现有 JS `retry.ts` 的能力：
```rust
async fn with_retry<F, T>(f: F, max_retries: u32, timeout_secs: u32) -> Result<T, String>
where F: Fn() -> Future<Output = Result<T, String>>
{
    // 最多重试 max_retries 次
    // 每次调用设置 timeout_secs 超时
    // 失败后立即重试（不等待）
}
```

超时和重试次数从 `AdvancedConfig` 读取。

---

## TaskManager 设计

### 状态机

```
Recording -> Transcribing -> Optimizing(可选) -> Pasting -> Done
                |                |                        |
              Failed           Failed                   Failed
```

### Task 结构体

```rust
struct Task {
    id: u32,              // 递增整数（与 Bubble 显示兼容）
    status: TaskStatus,
    audio_path: PathBuf,  // WAV 文件磁盘路径
    transcript: Option<String>,
    optimized: Option<String>,
    error: Option<String>,
    created_at: i64,
}
```

注意：Task ID 使用递增整数，因为 Bubble 窗口在 40x40 像素气泡中显示任务序号。

TaskManager 作为 `Arc<Mutex<TaskManager>>` 存在 Tauri State 中。

### 核心流程

```
1. shortcut.rs 录音完成 -> 拿到 base64 音频
2. 通过 AppHandle 获取 TaskManager State
3. TaskManager.process(app_handle, audio_base64):
   a. 分配递增 task_id
   b. 保存音频到磁盘 history/audio/{id}.wav
   c. 调用 bubble::show(app, task_id) 显示气泡
   d. tauri::async_runtime::spawn 异步执行:
      - 调用 ai::transcribe(audio, config) -> 文本
      - bubble::update(app, task_id, "transcribing")
      - 如果开启优化:
        - bubble::update(app, task_id, "optimizing")
        - 调用 ai::optimize(text, config) -> 优化文本
      - clipboard::paste_text(final_text)
      - bubble::update(app, task_id, "done")
      - bubble::hide(app, task_id, 2000)
      - 更新历史记录
```

使用 `tauri::async_runtime::spawn` 而非 `tokio::spawn`（tokio 不是直接依赖）。

### Bubble 交互

**无需新事件协议**。直接复用现有 `bubble.rs` 的函数：
- `bubble::show(app, task_id)` — 创建气泡，初始状态 recording
- `bubble::update(app, task_id, status)` — 更新状态（transcribing/optimizing/done/error）
- `bubble::hide(app, task_id, delay_ms)` — 延迟关闭

这些函数已经是 Rust 端实现的，TaskManager 内部直接调用即可。

### 历史记录 (`task/history.rs`)

沿用现有 JS `HistoryManager` 的磁盘存储方案：
- 最大记录数：可配置（当前默认 3 条）
- 音频存储：`app_data_dir/history/audio/{id}.wav`（独立文件）
- 元数据：`app_data_dir/history/history.json`（只存文本和状态，不含音频数据）
- 超出限制时自动删除旧记录及其音频文件

Tauri 命令：
- `get_history` — 读取 history.json（已有，改为从 TaskManager State 获取）
- `retry_task` — 读取磁盘音频文件，重新走管道（替代现有 `retry_record` 的事件转发）
- `clear_history` — 删除历史目录（新增）

---

## 现有文件变更

### `src-tauri/src/shortcut.rs` — 核心改造

录音完成后不再 `emit("recording-complete")`，改为直接调用 Rust TaskManager：

```rust
// 之前:
let _ = app_handle.emit("recording-complete", json!({ "audio": base64_audio }));

// 之后:
let task_mgr = app_handle.state::<Arc<Mutex<TaskManager>>>();
task_mgr.lock().unwrap().process(app_handle.clone(), base64_audio);
```

同理，`recording-started` 事件也不再需要（原来是给 JS Worker 创建 Task 用的），改为在 `process()` 内部处理。

`recording-error` 事件可保留用于日志或移除。

### `src-tauri/src/commands.rs` — 命令调整

| 命令 | 变更 |
|------|------|
| `show_bubble` | 从公开命令改为内部调用（JS 不再直接调用） |
| `update_bubble` | 同上 |
| `hide_bubble` | 同上 |
| `paste_text` | 同上（TaskManager 内部调用 clipboard 模块） |
| `proxy_request` | 保留（Settings 可能用于测试代理连通性） |
| `retry_record` | 改造：不再 emit 事件给 Worker，直接调用 TaskManager.retry() |
| `get_history` | 改造：从 TaskManager State 获取，不再直接读文件 |
| `clear_history` | 新增 |

从 `invoke_handler` 中移除不再公开的命令：`show_bubble`、`update_bubble`、`hide_bubble`、`paste_text`。

### `src-tauri/src/lib.rs` — 启动流程

```rust
// 新增:
mod ai;
mod task;

// setup 中:
// 1. 初始化 TaskManager 并注册为 State
let task_manager = Arc::new(Mutex::new(TaskManager::new(app.handle().clone())));
app.manage(task_manager);

// 2. 删除 Worker 窗口相关代码（第 71-78 行）:
// -- 删除 --
// if let Some(win) = app.get_webview_window("worker") {
//     let _ = win.set_position(...);
//     let _ = win.show();
// }
```

### `src-tauri/src/config/types.rs` — 默认值更新

```rust
// 第 86 行: "gemini-2.5-flash" -> "gemini-3-flash-preview"
model: "gemini-3-flash-preview".to_string(),

// 第 109 行: "gemini-2.5-flash" -> "gemini-3-flash-preview"
gemini_model: "gemini-3-flash-preview".to_string(),
```

### `src-tauri/tauri.conf.json` — 窗口配置

删除 `worker` 窗口定义。保留 `settings` 窗口。

### `src-tauri/Cargo.toml` — 依赖变更

```toml
# 现有 reqwest 0.13.2 已够用，只需确认 features 足够:
reqwest = { version = "0.13.2", features = ["json"] }
# 如需 HTTPS（Gemini/Qwen API），可能需要加 rustls-tls:
# reqwest = { version = "0.13.2", features = ["json", "rustls-tls"] }

# 无需新增 tokio（使用 tauri::async_runtime）
# 无需新增 uuid（使用递增整数 ID）
```

---

## 前端变更

### 删除的文件（完整列表）

```
src/views/worker/main.ts         -- Worker 入口
src/core/task-manager.ts         -- JS 任务管理器
src/core/transcribe.ts           -- JS 转录逻辑
src/core/optimize.ts             -- JS 优化逻辑
src/core/providers/gemini.ts     -- JS Gemini 客户端
src/core/providers/qwen-omni.ts  -- JS Qwen 客户端
src/core/providers/openai-compat.ts -- JS OpenAI 兼容客户端
src/core/providers/types.ts      -- Provider 接口定义
src/core/proxy-fetch.ts          -- JS 端代理封装
src/core/retry.ts                -- JS 重试逻辑
src/core/base64.ts               -- JS Base64 工具
src/core/prompt-loader.ts        -- JS 提示词加载
src/core/history.ts              -- JS 历史管理
worker.html                      -- Worker HTML 入口
```

### 保留但需修改的文件

```
src/core/types.ts                -- 保留 AppConfig 等 Settings 页面需要的类型
                                    删除 TaskStatus、HistoryRecord 等迁移到 Rust 的类型
```

### 保留不变的文件

```
src/main.tsx                     -- Settings 入口
src/views/bubble/main.ts         -- Bubble 入口（无需改动，已监听 update-bubble 事件）
src/pages/                       -- Settings 页面
src/components/                  -- UI 组件
src/lib/                         -- Tauri API 封装
```

### Settings 页面改动

- 历史记录页面：调用 Tauri 命令 `get_history` / `retry_task` / `clear_history`（不再通过 JS TaskManager）
- 模型选择：移除 `gemini-2.5-flash` 选项
- 历史更新监听：从 `history-updated` 事件改为主动查询（或保留事件，由 Rust TaskManager emit）

### npm 依赖清理 (`package.json`)

以下包不再需要：
- `@google/genai` — Gemini SDK
- `openai` — OpenAI 兼容调用（如果有的话）

---

## 启动流程对比

### 之前

```
App 启动 -> 创建 Settings 窗口(隐藏) -> 创建 Worker 窗口(1x1) -> 闪一下
         -> Worker 加载 JS -> TaskManager 初始化
         -> 注册快捷键 -> 就绪
```

### 之后

```
App 启动 -> 创建 Settings 窗口(隐藏) -> 无 Worker 窗口，不闪
         -> Rust TaskManager 初始化（纯内存，毫秒级）
         -> 注册快捷键 -> 就绪
```

---

## 风险与注意事项

- Gemini 3 系列模型处于 preview 阶段，API 格式可能变化
- reqwest 0.13.2 需确认默认 TLS 后端在 macOS/Windows 上工作正常，必要时加 `rustls-tls` feature
- 初期使用非流式 API，对长音频可能有首字节延迟，后续可加回流式
- 历史音频存储在磁盘上，需注意磁盘空间（3 条记录约 3MB，可接受）
- `shortcut.rs` 改造时注意 `Mutex` 锁不能跨 `.await`，需要在 spawn 之前释放锁
- 代理配置需同时支持 HTTP 和 SOCKS5（reqwest 原生支持）
