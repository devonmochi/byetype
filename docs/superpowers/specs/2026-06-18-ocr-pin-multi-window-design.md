# OCR 结果窗口钉住多开设计

日期：2026-06-18
状态：已确认，待实现

## 背景与目标

ByeType 的 OCR 结果窗口（预览窗）当前是强单例：窗口标签写死为 `preview`，配 `PINNED`、`PREVIEW_READY`、`BLUR_HANDLER_REGISTERED` 等一组全局 static 状态。每次识别都复用同一个窗口、替换内容，「钉住」只是阻止该窗失焦时自动关闭，且每次新识别都会把 `PINNED` 重置为 false。

目标：让被钉住的结果窗原地保留，再次识别时新结果另开新窗，两者并存，可同时存在任意多个钉住窗。

## 已确认的产品规则

- 钉住数量不限，由用户自行关闭
- 没钉住的窗口再次识别时原地替换（沿用现状）
- 钉住的窗口再次识别时保持不动，新结果另开新窗
- 新窗口跟随光标/截图区域附近定位（沿用现状定位逻辑）

一句话规则：**活动槽里的窗随用随换；一旦钉住就毕业留底，活动槽腾空给下一个。**

## 架构：活动槽加钉住毕业

核心矛盾：Tauri 窗口标签创建后不可改名，要让多个结果窗同时存在，必须从单标签单例改造为多标签加每窗独立状态。

三个核心概念：

- 序号标签：窗口标签从写死的 `preview` 改为 `preview-{N}`，N 自增，永不复用旧号
- 活动槽：全局只记一个「当前临时窗」的标签 `ACTIVE_LABEL`，没钉住时复用它、替换内容
- 钉住毕业：点钉住等于把活动槽里这个窗毕业成钉住窗，活动槽清空，该窗留在原地，下次识别建新活动窗

## 状态模型改造

现有全局 static 是单例的根，逐项改造：

- `PINNED: AtomicBool`：删除。钉住状态变为每窗一份，由前端各自持有，后端按标签记录
- `ACTIVE_LABEL`：新增 `Mutex<Option<String>>`，记当前活动窗标签，钉住或关闭时置 None
- `WINDOW_SEQ`：新增 `AtomicU64`，自增生成 `preview-{N}`
- `PREVIEW_READY`：从单一 bool 改为 `Mutex<HashMap<String, bool>>`，按标签存
- `CREATED_AT`：改为按标签记录创建时间，用于 blur 宽限期判断
- `BLUR_HANDLER_REGISTERED`：删除。每个新窗创建时各自注册一次 blur，天然只注册一次

## 关键流程

### show(text) 显示识别结果

1. 读 `ACTIVE_LABEL`：有活动窗则复用它，`set_size` 加 emit 新文本替换（沿用现状逻辑）
2. 无活动窗（首次、上一个已毕业钉住、或已关闭）则生成 `preview-{N}` 新建窗口，写入 `ACTIVE_LABEL`
3. 定位、show、blur 监听照旧，全部按这个具体标签操作

### 钉住毕业

1. 前端点钉住，带上自己的窗口标签调 `set_preview_pinned(label, true)`
2. 后端：若该 label 等于 `ACTIVE_LABEL`，把 `ACTIVE_LABEL` 置 None（毕业，活动槽腾空）
3. 该窗 pinned 记为 true，失焦不再自动关
4. 取消钉住：pinned 记 false。不抢回活动槽，失焦时按未钉住规则关闭

## 关键坑：事件广播串台

现有前端用全局 `listen('preview-text')` 与 `emit('preview-text-applied')`。Tauri 的 `emit` 是全局广播，一旦同时存在两个以上 preview 窗口，后端给新窗发的文本会被所有窗口收到，钉住窗内容会被冲掉。这是本方案必须同时解决的问题，否则多开即串台。

解法是按窗口定向通信：

- 后端发给某个窗：用 `window.emit_to(label, event, payload)` 替代全局 `emit`
- 前端回执：`preview-ready` 与 `preview-text-applied` 带上自己的 `getCurrentWindow().label`，后端按标签区分
- 前端监听：改用 `getCurrentWindow().listen(...)` 只收发本窗事件，而非全局 `listen`

## 失焦关闭与剪贴板

每个窗口的 blur 处理按自己的标签判断：

- 未钉住且过了 800ms 宽限期则关闭自己（按标签 close，不再写死 `get_webview_window("preview")`）
- 已钉住则不关（沿用现状）
- 窗口 Destroyed 时从各 HashMap 清掉自己的标签项，若自己是 `ACTIVE_LABEL` 则置 None

前端 `handleBlur` 写剪贴板、`handleClose` 关闭都改为针对当前窗（`close_preview_window(label)`），逻辑不变。

## 预热处理

预热为掩盖冷启动，多实例下保持只预热「下一个活动窗」：

- prewarm 时若 `ACTIVE_LABEL` 已有则跳过（幂等，沿用现状）
- 否则建一个 `preview-{N}` 隐藏窗，直接写入 `ACTIVE_LABEL` 占位，等 show 复用
- 钉住毕业后活动槽空了，下次识别走 prewarm 或新建，自然又预热下一个。预热只服务活动窗，钉住窗不预热

## 改动文件清单

- `src-tauri/src/preview.rs`：主战场，状态模型（static 改 Mutex/HashMap）、序号标签、活动槽、emit 改 emit_to、按标签 blur/close/prewarm
- `src/views/preview/App.tsx`：listen 改按窗 listen、回执带 label、钉住带 label、关闭带 label
- `src-tauri/src/task/mod.rs`：调用点 show / prewarm 签名基本不变，确认无写死 `preview` 引用
- 其它引用 `preview` 标签处：全局搜 `"preview"` 字面量，改为按活动槽或标签解析（如 `close_if_exists`）

## 测试要点

- 不钉住连续识别：始终复用同一窗替换（回归现状，不能退化）
- 钉住 A 后再识别：A 原地不动、内容不变，B 新开
- 钉住 A、钉住 B、再识别 C：A 与 B 都在，C 是新活动窗
- 串台验证：钉住 A 后识别 B，确认 A 内容没被 B 冲掉（验证 emit_to 生效）
- 钉住窗失焦不关，未钉住窗失焦正常关且清理状态
- 关闭活动窗后再识别：能正常新建
