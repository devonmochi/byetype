# Rust AI Pipeline Migration - Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Migrate the entire AI pipeline (transcription, optimization, task management, history) from JS Worker to Rust, eliminating the Worker window and fixing startup flash/focus issues.

**Architecture:** All AI HTTP calls move to Rust via `reqwest`. A Rust `TaskManager` replaces the JS one, directly calling `bubble::show/update/hide` and `clipboard::paste_text`. The Worker window and all JS `src/core/` modules are deleted. Settings UI and Bubble UI remain as-is with minimal changes.

**Tech Stack:** Rust, reqwest 0.13.2, serde_json, tauri::async_runtime, tokio (time), Tauri 2 events

**Spec document:** `DESIGN-rust-ai-pipeline.md` (project root)

---

## File Structure

### New Rust files (create)

| File | Responsibility |
|------|---------------|
| `src-tauri/src/ai/mod.rs` | AI module entry: `transcribe()` and `optimize()` public functions |
| `src-tauri/src/ai/types.rs` | Shared request/response structs for all providers |
| `src-tauri/src/ai/gemini.rs` | Gemini 3 REST API client (generateContent endpoint) |
| `src-tauri/src/ai/qwen.rs` | Qwen3-Omni client (OpenAI-compat chat/completions) |
| `src-tauri/src/ai/openai_compat.rs` | Generic OpenAI-compat client for text optimization |
| `src-tauri/src/ai/prompt.rs` | Prompt loading and assembly (port of prompt-loader.ts) |
| `src-tauri/src/ai/retry.rs` | `with_retry` async utility (port of retry.ts) |
| `src-tauri/src/task/mod.rs` | TaskManager state machine + process/retry logic |
| `src-tauri/src/task/history.rs` | History records: disk persistence, audio file management |

### Existing Rust files (modify)

| File | Changes |
|------|--------|
| `src-tauri/src/lib.rs` | Add `mod ai; mod task;`, init TaskManager as State, remove Worker window code |
| `src-tauri/src/shortcut.rs` | Replace `emit("recording-complete")` with `TaskManager.process()` call |
| `src-tauri/src/commands.rs` | Rewrite `retry_record`/`get_history`, add `clear_history`, remove bubble/paste from invoke_handler |
| `src-tauri/src/config/types.rs` | Update default model from `gemini-2.5-flash` to `gemini-3-flash-preview` |
| `src-tauri/tauri.conf.json` | Remove `worker` window definition |
| `src-tauri/Cargo.toml` | Add `tokio` with `time` feature |

### Frontend files (modify)

| File | Changes |
|------|--------|
| `src/views/settings/tabs/TranscribeTab.tsx` | Remove `gemini-2.5-flash` from MODELS, remove `is25Flash` branch |
| `src/views/settings/tabs/OptimizeTab.tsx` | Remove `gemini-2.5-flash` option, remove `is25Flash` branch |
| `src/core/types.ts` | Remove `gemini-2.5-flash` from model union. Keep `HistoryRecord`, `TaskStatus`, `RetryStatusUpdate` (still used by HistoryTab) |
| `src/lib/tauri-api.ts` | Remove bubble/paste/recording-event wrappers (no longer called from JS) |
| `vite.config.ts` | Remove `worker` entry from `rollupOptions.input` |

### Files to delete

```
src/views/worker/main.ts
src/core/task-manager.ts
src/core/transcribe.ts
src/core/optimize.ts
src/core/providers/gemini.ts
src/core/providers/qwen-omni.ts
src/core/providers/openai-compat.ts
src/core/providers/types.ts
src/core/proxy-fetch.ts
src/core/retry.ts
src/core/base64.ts
src/core/prompt-loader.ts
src/core/history.ts
worker.html
```

---

## Task 1: AI Types, Retry, and Cargo.toml

**Files:**
- Create: `src-tauri/src/ai/types.rs`
- Create: `src-tauri/src/ai/retry.rs`
- Modify: `src-tauri/Cargo.toml` (add tokio)

- [ ] **Step 1: Add tokio dependency**

> **Note:** The spec says "无需新增 tokio", but `tauri::async_runtime` does not re-export `tokio::time::timeout`. We need tokio as a direct dependency for the `time` feature only.

In `src-tauri/Cargo.toml`, add after line 34 (reqwest):
```toml
tokio = { version = "1", features = ["time"] }
```

- [ ] **Step 2: Create `src-tauri/src/ai/types.rs`**

```rust
use serde::{Deserialize, Serialize};

// === Gemini types ===

#[derive(Serialize)]
pub struct GeminiRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_instruction: Option<GeminiContent>,
    pub contents: Vec<GeminiContent>,
    #[serde(rename = "generationConfig", skip_serializing_if = "Option::is_none")]
    pub generation_config: Option<GeminiGenerationConfig>,
}

#[derive(Serialize)]
pub struct GeminiContent {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    pub parts: Vec<GeminiPart>,
}

#[derive(Serialize)]
#[serde(untagged)]
pub enum GeminiPart {
    Text { text: String },
    InlineData {
        #[serde(rename = "inlineData")]
        inline_data: GeminiInlineData,
    },
}

#[derive(Serialize)]
pub struct GeminiInlineData {
    #[serde(rename = "mimeType")]
    pub mime_type: String,
    pub data: String,
}

#[derive(Serialize)]
pub struct GeminiGenerationConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking: Option<GeminiThinkingConfig>,
}

#[derive(Serialize)]
#[serde(untagged)]
pub enum GeminiThinkingConfig {
    Level {
        #[serde(rename = "thinkingLevel")]
        thinking_level: String,
    },
}

#[derive(Deserialize)]
pub struct GeminiResponse {
    pub candidates: Option<Vec<GeminiCandidate>>,
}

#[derive(Deserialize)]
pub struct GeminiCandidate {
    pub content: Option<GeminiResponseContent>,
}

#[derive(Deserialize)]
pub struct GeminiResponseContent {
    pub parts: Option<Vec<GeminiResponsePart>>,
}

#[derive(Deserialize)]
pub struct GeminiResponsePart {
    pub text: Option<String>,
}

// === OpenAI-compat types (Qwen + optimize) ===

#[derive(Serialize)]
pub struct ChatCompletionRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modalities: Option<Vec<String>>,
}

#[derive(Serialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: ChatContent,
}

#[derive(Serialize)]
#[serde(untagged)]
pub enum ChatContent {
    Text(String),
    Parts(Vec<ChatContentPart>),
}

#[derive(Serialize)]
#[serde(tag = "type")]
pub enum ChatContentPart {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "input_audio")]
    InputAudio { input_audio: AudioData },
}

#[derive(Serialize)]
pub struct AudioData {
    pub data: String,
    pub format: String,
}

#[derive(Deserialize)]
pub struct ChatCompletionResponse {
    pub choices: Option<Vec<ChatChoice>>,
}

#[derive(Deserialize)]
pub struct ChatChoice {
    pub message: Option<ChatResponseMessage>,
}

#[derive(Deserialize)]
pub struct ChatResponseMessage {
    pub content: Option<String>,
}
```

- [ ] **Step 3: Create `src-tauri/src/ai/retry.rs`**

```rust
use std::future::Future;
use std::time::Duration;

pub async fn with_retry<F, Fut, T>(
    f: F,
    max_retries: u32,
    timeout_secs: u32,
) -> Result<T, String>
where
    F: Fn() -> Fut,
    Fut: Future<Output = Result<T, String>>,
{
    let timeout_duration = Duration::from_secs(timeout_secs as u64);

    for attempt in 0..=max_retries {
        match tokio::time::timeout(timeout_duration, f()).await {
            Ok(Ok(result)) => return Ok(result),
            Ok(Err(e)) => {
                if attempt >= max_retries {
                    return Err(e);
                }
                eprintln!("[AI] Attempt {} failed: {}, retrying...", attempt + 1, e);
            }
            Err(_) => {
                if attempt >= max_retries {
                    return Err("Request timed out".to_string());
                }
                eprintln!("[AI] Attempt {} timed out, retrying...", attempt + 1);
            }
        }
    }
    Err("All retries exhausted".to_string())
}
```

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/ai/types.rs src-tauri/src/ai/retry.rs src-tauri/Cargo.toml
git commit -m "feat: add AI types and retry utility"
```

---

## Task 2: Gemini 3 Client

**Files:**
- Create: `src-tauri/src/ai/gemini.rs`

- [ ] **Step 1: Create `src-tauri/src/ai/gemini.rs`**

See `DESIGN-rust-ai-pipeline.md` "Gemini 3 客户端" section. Key points:
- URL: `https://generativelanguage.googleapis.com/v1beta/models/{model}:generateContent?key={api_key}`
- System instruction as separate field, audio as `inlineData`
- Thinking uses `thinkingLevel` (Gemini 3 only)
- `extract_gemini_text()` takes last text part (skipping thinking parts)
- Both `transcribe()` and `optimize()` functions
- Optimize merges text+systemPrompt in user content: `<voice-input>\n{text}\n</voice-input>\n\n{systemPrompt}`

- [ ] **Step 2: Commit**

```bash
git add src-tauri/src/ai/gemini.rs
git commit -m "feat: add Gemini 3 REST API client"
```

---

## Task 3: Qwen3 Client

**Files:**
- Create: `src-tauri/src/ai/qwen.rs`

- [ ] **Step 1: Create `src-tauri/src/ai/qwen.rs`**

Key points (from `src/core/providers/qwen-omni.ts`):
- URL: `https://dashscope.aliyuncs.com/compatible-mode/v1/chat/completions`
- Auth: `Bearer {api_key}` header
- Message content: array with `text` (systemPrompt) + `input_audio` (data URI `data:;base64,{b64}` — no MIME type, matches existing JS behavior)
- `modalities: ["text"]`
- Non-streaming (JS used streaming but we simplify)

- [ ] **Step 2: Commit**

```bash
git add src-tauri/src/ai/qwen.rs
git commit -m "feat: add Qwen3-Omni REST API client"
```

---

## Task 4: OpenAI-Compat Client

**Files:**
- Create: `src-tauri/src/ai/openai_compat.rs`

- [ ] **Step 1: Create `src-tauri/src/ai/openai_compat.rs`**

Key points (from `src/core/providers/openai-compat.ts`):
- URL: `{base_url}/chat/completions`
- Auth: `Bearer {api_key}` header
- Single user message: `<voice-input>\n{text}\n</voice-input>\n\n{systemPrompt}` (matches existing JS behavior)
- Reads config from `OpenAICompatConfig` struct

- [ ] **Step 2: Commit**

```bash
git add src-tauri/src/ai/openai_compat.rs
git commit -m "feat: add OpenAI-compat client for text optimization"
```

---

## Task 5: Prompt Loader

**Files:**
- Create: `src-tauri/src/ai/prompt.rs`

- [ ] **Step 1: Create `src-tauri/src/ai/prompt.rs`**

Port of `src/core/prompt-loader.ts`. Key functions:
- `build_transcribe_prompt(config, prompts_dir)` — loads agent.md, vocabulary.md, rules.md, wraps each in `<document name="...">` tags, joins with `\n\n`
- `load_optimize_prompt(config, prompts_dir)` — loads text-optimize.md wrapped in `<document>` tag
- `resolve_prompt_path(custom, builtin)` — returns custom if non-empty, else builtin
- Prompt dir resolution: uses prompts_dir passed from TaskManager (resolved at startup via `resolve_prompts_dir`)

- [ ] **Step 2: Commit**

```bash
git add src-tauri/src/ai/prompt.rs
git commit -m "feat: add prompt loader"
```

---

## Task 6: AI Module Entry (mod.rs)

**Files:**
- Create: `src-tauri/src/ai/mod.rs`
- Modify: `src-tauri/src/lib.rs` (add `mod ai;`)

- [ ] **Step 1: Create `src-tauri/src/ai/mod.rs`**

Must include all sub-module declarations:
```rust
pub mod types;
pub mod retry;
pub mod gemini;
pub mod qwen;
pub mod openai_compat;
pub mod prompt;
```

Public functions with explicit `client` parameter (client built once per pipeline run, not per request):
```rust
pub async fn transcribe(client: &reqwest::Client, audio_base64: &str, config: &AppConfig, prompts_dir: &Path) -> Result<String, String>
pub async fn optimize(client: &reqwest::Client, text: &str, config: &AppConfig, prompts_dir: &Path) -> Result<String, String>
```

Dispatch logic:
- `config.transcribe.model == "qwen3-omni-flash"` → `qwen::transcribe()`
- Otherwise → `gemini::transcribe()`
- `config.optimize.optimize_type == "gemini"` → `gemini::optimize()`
- Otherwise → `openai_compat::optimize()`

- [ ] **Step 2: Add `mod ai;` to `lib.rs` line 1**

- [ ] **Step 3: Verify: `cd src-tauri && cargo check`**

Expected: compiles (task module not yet wired)

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/ai/mod.rs src-tauri/src/lib.rs
git commit -m "feat: add AI module entry with provider dispatch"
```

---

## Task 7: History Manager

**Files:**
- Create: `src-tauri/src/task/history.rs`

- [ ] **Step 1: Create `src-tauri/src/task/history.rs`**

Port of `src/core/history.ts`. Key points:
- `HistoryRecord` struct matches JS `HistoryRecord` exactly (camelCase serde)
- `HistoryManager::new(data_dir)` — sets up paths under `{data_dir}/history/`
- `init()` — creates dirs, loads JSON, validates audio paths, cleans orphans
- `add_record(audio_b64, transcribe, optimize, status, error)` — saves WAV to disk, trims to MAX_RECORDS=3
- `update_record(id, ...)` — for retry updates
- `get_audio_base64(id)` — reads WAV from disk, returns base64
- `clear()` — removes history dir and recreates
- Uses `base64::Engine` trait for encode/decode
- ISO timestamp via manual date calculation (no chrono dep)

- [ ] **Step 2: Commit**

```bash
git add src-tauri/src/task/history.rs
git commit -m "feat: add history manager with disk persistence"
```

---

## Task 8: TaskManager

**Files:**
- Create: `src-tauri/src/task/mod.rs`
- Modify: `src-tauri/src/lib.rs` (add `mod task;`)

- [ ] **Step 1: Create `src-tauri/src/task/mod.rs`**

Port of `src/core/task-manager.ts` + `src/views/worker/main.ts`. Key points:
- `TaskManager` struct: `task_counter`, `active_count`, `history`, `prompts_dir`
- `SharedTaskManager = Arc<Mutex<TaskManager>>` registered as Tauri State
- `process_recording(app, audio_base64)` — allocates task_id, shows bubble with initial status `"transcribing"` (not `"recording"` — recording is already done at this point), spawns async pipeline
- `retry_record(app, record_id)` — reads audio from history, spawns pipeline with retry_record_id
- `run_pipeline(app, task_id, audio, retry_id)` async — transcribe → optimize → paste → history
- Uses `crate::bubble::show/update/hide` directly (no events)
- Uses `crate::clipboard::paste_text` directly
- Emits `"history-updated"` and `"retry-status"` events for Settings UI
- Builds reqwest client with proxy from config

- [ ] **Step 2: Add `mod task;` to `lib.rs`**

- [ ] **Step 3: Verify: `cd src-tauri && cargo check`**

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/task/mod.rs src-tauri/src/lib.rs
git commit -m "feat: add Rust TaskManager with AI pipeline"
```

---

## Task 9: Integration — lib.rs, shortcut.rs, commands.rs

**Files:**
- Modify: `src-tauri/src/lib.rs` (full rewrite of setup block)
- Modify: `src-tauri/src/shortcut.rs` (replace emit with TaskManager call)
- Modify: `src-tauri/src/commands.rs` (rewrite history/retry, remove bubble/paste commands)

- [ ] **Step 1: Rewrite `lib.rs`**

Changes:
- Remove `paste_text`, `show_bubble`, `update_bubble`, `hide_bubble` from `invoke_handler`
- In setup: init TaskManager with `app_data_dir` + `resolve_prompts_dir`, manage as State
- Remove Worker window block (lines 71-78)

- [ ] **Step 2: Add `resolve_prompts_dir_pub` to `commands.rs`**

Public wrapper for the private `resolve_prompts_dir` function, called from `lib.rs` setup.

- [ ] **Step 3: Modify `shortcut.rs`**

Replace `app_handle.emit("recording-complete", ...)` (line 28-30) with:
```rust
crate::task::process_recording(&app_handle, base64_audio);
```
Replace `app_handle.emit("recording-started", ())` (line 43) with empty block (just a comment).

**Keep `use tauri::Emitter;`** — the two `recording-error` emit calls (lines 34-35 and 47-48) still use it. These are harmless and useful for debug logging. Do NOT remove the Emitter import.

- [ ] **Step 4: Rewrite `commands.rs` history/retry/clear**

- `get_history`: read from `SharedTaskManager` state, not from disk
- `retry_record`: call `crate::task::retry_record(&app, record_id)` instead of emit
- Remove `paste_text`, `show_bubble`, `update_bubble`, `hide_bubble` command functions

> Note: `clear_history` is deferred — no frontend UI calls it yet. Can be added later.

- [ ] **Step 5: Verify: `cd src-tauri && cargo check`**

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/lib.rs src-tauri/src/shortcut.rs src-tauri/src/commands.rs
git commit -m "feat: wire TaskManager into lib/shortcut/commands"
```

---

## Task 10: Config & Window Cleanup

**Files:**
- Modify: `src-tauri/src/config/types.rs:86,109`
- Modify: `src-tauri/tauri.conf.json:23-33`

- [ ] **Step 1: Update default models in `config/types.rs`**

Line 86: `"gemini-2.5-flash"` → `"gemini-3-flash-preview"`
Line 109: `"gemini-2.5-flash"` → `"gemini-3-flash-preview"`

- [ ] **Step 2: Remove Worker window from `tauri.conf.json`**

Delete the entire worker window object (lines 23-33).

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/config/types.rs src-tauri/tauri.conf.json
git commit -m "chore: update default models, remove Worker window config"
```

---

## Task 11: Delete JS Core Files

- [ ] **Step 1: Remove `worker` entry from `vite.config.ts`**

In `vite.config.ts` line 15, remove:
```typescript
worker: resolve(__dirname, 'worker.html'),
```

- [ ] **Step 2: Delete files**

```bash
rm src/views/worker/main.ts
rm src/core/task-manager.ts src/core/transcribe.ts src/core/optimize.ts
rm src/core/providers/gemini.ts src/core/providers/qwen-omni.ts src/core/providers/openai-compat.ts src/core/providers/types.ts
rm src/core/proxy-fetch.ts src/core/retry.ts src/core/base64.ts src/core/prompt-loader.ts src/core/history.ts
rm worker.html
rmdir src/core/providers src/views/worker 2>/dev/null || true
```

- [ ] **Step 2: Commit**

```bash
git add -A
git commit -m "chore: delete JS Worker and core modules migrated to Rust"
```

---

## Task 12: Update Settings UI

**Files:**
- Modify: `src/views/settings/tabs/TranscribeTab.tsx`
- Modify: `src/views/settings/tabs/OptimizeTab.tsx`
- Modify: `src/core/types.ts`
- Modify: `src/lib/tauri-api.ts`

- [ ] **Step 1: TranscribeTab — remove gemini-2.5-flash**

- Remove `{ value: 'gemini-2.5-flash', label: 'Gemini 2.5 Flash' }` from MODELS (line 13)
- Remove `const is25Flash = transcribe.model === 'gemini-2.5-flash'` (line 22)
- Replace the `is25Flash ? (budget input) : (level select)` ternary (lines 84-109) with just the level select

- [ ] **Step 2: OptimizeTab — remove gemini-2.5-flash**

- Remove `const is25Flash = ...` (line 27)
- Remove `<option value="gemini-2.5-flash">` (line 78)
- Replace `is25Flash ? (budget) : (level)` ternary (lines 90-116) with just level select

- [ ] **Step 3: types.ts — update model union**

Line 16: remove `'gemini-2.5-flash' |`

> Keep `HistoryRecord`, `TaskStatus`, `RetryStatusUpdate` types — `HistoryTab.tsx` still imports them.

- [ ] **Step 4: tauri-api.ts — remove unused wrappers**

Remove: `showBubble`, `updateBubble`, `hideBubble`, `pasteText`, `onRecordingStarted`, `onRecordingComplete`, `onRecordingError`

- [ ] **Step 5: Commit**

```bash
git add src/views/settings/tabs/TranscribeTab.tsx src/views/settings/tabs/OptimizeTab.tsx src/core/types.ts src/lib/tauri-api.ts
git commit -m "chore: remove gemini-2.5-flash from UI, clean unused wrappers"
```

---

## Task 13: npm Dependency Cleanup

- [ ] **Step 1: Uninstall packages**

```bash
npm uninstall @google/genai openai
```

- [ ] **Step 2: Verify frontend builds: `npm run build`**

- [ ] **Step 3: Commit**

```bash
git add package.json package-lock.json
git commit -m "chore: remove @google/genai and openai npm deps"
```

---

## Task 14: Full Build & Smoke Test

- [ ] **Step 1: `cd src-tauri && cargo check`** — no errors
- [ ] **Step 2: `npm run build`** — no errors
- [ ] **Step 3: `npm run tauri dev`** — app starts clean, no flash
- [ ] **Step 4: Test recording** — F4 → record → F4 → bubble → transcribe → paste
- [ ] **Step 5: Test retry** — Settings → History → click retry
- [ ] **Step 6: Commit any fixes**

```bash
git add -A
git commit -m "fix: address smoke test issues"
```
