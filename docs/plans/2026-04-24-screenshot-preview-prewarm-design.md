# 截图预览窗口预热优化设计

日期: 2026-04-24
范围: macOS 平台,截图 OCR 流程的预览窗口体验优化

## 1. 背景与问题

触发截图快捷键 → 选区 → AI 思考 → 弹出预览文本框,这一链路中存在两处感知卡顿:

1. 按键到选区出现:`screencapture -i` 进程冷启动 100~400ms。**本次不修。**
2. AI 返回到预览框出现:每次都新建 WKWebView,首次加载 React + bundle 约 500~1000ms,表现为 AI 已返回但界面仍空白。**本次目标。**

用户感知:AI 思考时间是"正常等待",可以接受;但 AI 已经返回后又额外等近 1 秒才看到弹窗,这段空白特别明显。

## 2. 总体策略

采用**预热**而非**预创建常驻**:

- 在截图选区完成、AI 调用即将开始时,**并行**触发预览窗口的隐藏式创建。
- AI 调用本身耗时 2~5s,足以掩盖 webview 的 500~1000ms 创建成本。
- AI 返回时窗口已就绪,直接 emit 文本 + `window.show()`,几乎瞬时显示。
- AI 失败时显式关闭隐藏窗口,**零常驻内存代价**。

关键取舍:**零常驻内存** vs. 稍高实现复杂度(需处理失败清理、主线程调度)。选择前者,因 ByeType 是常驻后台应用,内存敏感。

仅影响 macOS 路径,Windows 走原生 Win32 overlay,不受影响。

## 3. 组件设计

### 3.1 新增 `preview::prewarm(app)`

职责:幂等地创建一个隐藏的 preview webview。

- 若 `preview` 窗口已存在 → 直接返回(幂等)
- 否则在主线程上 build 一个 `visible: false` 的窗口,让 React bundle 开始加载
- **不** emit 任何文本、**不** 触发 `window.show()`、**不** 绑定 blur 关闭事件(避免空窗口被误关)
- 失败只打 log,不 panic、不中断主流程(退化到旧的冷启动行为)

### 3.2 修改 `preview::show(app, text)`

开头判断窗口是否已由 prewarm 创建:

- 已存在 → 复用窗口,跳过 build,根据文本调整尺寸,绑定 blur 事件,emit + show
- 不存在 → 走原有创建逻辑(回退路径,行为与当前一致)

注意:`preview-ready` + `preview-text` 两段式握手已经天然支持"窗口先存在、文本后到"的时序,前端不需要改。

### 3.3 新增 `preview::close_if_exists(app)`

供失败路径调用,关闭可能残留的隐藏预览窗口。

### 3.4 修改 `task::run_extract_pipeline()`

- 在拿到 `image_base64`、进入 AI 调用前一行加 `crate::preview::prewarm(app)`
- `finish_extract_pipeline()` 中,当 `status != "completed"` 时调用 `crate::preview::close_if_exists(app)`

## 4. 数据流时序

```
选区完成
  │
  ├─► base64 编码 ──► spawn(preview::prewarm)  ◀── 并行
  │                        └─► 创建隐藏 webview,React 启动
  │
  └─► AI 调用 (2-5s,掩盖 webview 创建)
         │
         ▼
   AI 返回 ──► preview::show(text)
                 ├─► 复用窗口
                 ├─► 根据 text 调整尺寸
                 ├─► emit preview-text
                 └─► window.show()  ◀── 用户几乎瞬时看到弹窗
```

## 5. 错误处理与边界

| 边界场景 | 处理策略 |
|---------|---------|
| AI 失败/取消,窗口已预热 | `finish_extract_pipeline` 中 `close_if_exists` 关闭隐藏窗口 |
| 用户取消区域选择 (Esc) | 还未到预热点,无须处理 |
| 连续两次截图 | `max_parallel` 已限制并发;若并发,prewarm 幂等返回,共享窗口(行为与当前一致) |
| `prewarm` 本身失败 | 仅打 log;`show()` 走回退路径,等价旧行为 |
| `pinned` 状态污染 | 后端 `PINNED` 在 `show()` 已 reset;前端 React 每次新挂载默认 false,无污染 |

## 6. 实现关键约束

**Tauri `WebviewWindowBuilder::build()` 必须在主线程调用**。`prewarm` 内部需通过 `app.run_on_main_thread()` 分派,不能直接在 `tokio::spawn` 里跑。这是实现时最容易踩坑的点。

## 7. 文件改动清单

仅 2 个 Rust 文件:

- `src-tauri/src/preview.rs`:新增 `prewarm` + `close_if_exists`,重构 `show` 支持窗口复用
- `src-tauri/src/task/mod.rs`:`run_extract_pipeline` 调用 `prewarm`,`finish_extract_pipeline` 失败分支调用 `close_if_exists`

**不改动**:
- `preview.html` / `src/views/preview/App.tsx` — 现有握手已支持新时序
- `src-tauri/src/lib.rs` — 无新 command 注册
- Windows 平台路径

## 8. 验证方式

- **效果验证**:截图后观察 AI 返回到弹窗显示之间的空白时长,目标从 ~800ms 降至 < 100ms
- **退化验证**:临时注释掉 `prewarm` 调用,确认仍能正常弹窗(只是慢)
- **失败清理验证**:模拟 AI 失败,确认无残留隐藏窗口(下次截图仍能正常预热)
- **取消验证**:截图后取消任务,预览窗口不应残留

## 9. 复杂度估计

- 新增代码:约 30-40 行 Rust
- 重构代码:`show()` 约 20 行
- 主要风险:`run_on_main_thread` 调度时序、复用窗口分支的尺寸 resize 平滑度
