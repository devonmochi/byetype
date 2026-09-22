# ByeType 快捷键行为修改 v2.2 — 实现报告

> 需求书：`ByeType 快捷键行为修改需求规格说明书 v2.2（最终版）`
> 基线提交（实施前 HEAD）：`e97dbe09912762b4caf03e49c18ce9f33c829707`
> 修改范围：仅快捷键配置、捕获、保存、迁移、注册、注销与触发所必需的代码

## 1. 修改文件

| 文件 | 说明 |
|---|---|
| `src-tauri/src/config/types.rs` | 新增平台化默认值：Windows `shortcut` 默认 `AltRight`，其他平台保持 `F4`；`shortcut` 增加 serde default |
| `src-tauri/src/config/mod.rs` | `ConfigManager` 重构为 `raw JSON + AppConfig view`；加载时执行快捷键迁移并原子写盘；`update` 改为深合并、保护快捷键字段与 schema、保留未知字段；新增 `commit_shortcut_patch` |
| `src-tauri/src/config/shortcut_config.rs`（新增） | 纯逻辑：schema 解析、FutureSchemaMode、四字段缺失/null/空白/Backspace 规范化、Windows schema v1 / F4→AltRight 迁移、字段级 patch、未知字段合并 |
| `src-tauri/src/config/migration.rs` | 未改逻辑（模型 ID 迁移保留），由 `mod.rs` 调用 |
| `src-tauri/src/shortcut/model.rs`（新增） | 纯逻辑：字段模型、规范化（trim/别名/顺序/主键）、Backspace 语义、冲突图、候选校验；含单元测试 |
| `src-tauri/src/shortcut/mod.rs`（由 `shortcut.rs` 改为目录模块） | 运行时：注册/注销、RegistrationPlan、registration generation、capture token 会话、串行调度层、`update_shortcuts` 字段级更新与回滚；语音/PTT 业务逻辑原样迁入 |
| `src-tauri/src/shortcut/native/mod.rs`（新增） | AltRight 后端抽象（单后端约束） |
| `src-tauri/src/shortcut/native/windows.rs`（新增） | Windows Raw Input（首选）+ `WH_KEYBOARD_LL`（备用），仅识别物理右 Alt |
| `src-tauri/src/commands.rs` | `save_config` 改为保留快捷键域并委托专属路径；新增 `update_shortcuts`、`reset_shortcuts` 与 capture token 命令、`get_shortcut_status` |
| `src-tauri/src/lib.rs` | 注册 `ShortcutManager` 与新增命令；启动注册失败不再 panic |
| `src-tauri/Cargo.toml` | 仅新增 `windows-sys` 的 `Win32_System_Threading` feature |
| `src/core/types.ts`、`src/lib/tauri-api.ts` | 新增快捷键命令与类型 |
| `src/views/settings/components/ShortcutInput.tsx`（新增） | 前端捕获状态机（Idle/Requesting/Capturing/Committing/Cancelling）、AltRight pending、Backspace、Escape/Tab、blur/visibility/unmount 清理 |
| `src/views/settings/tabs/GeneralTab.tsx`、`src/views/settings/App.tsx` | 使用 `ShortcutInput`；快捷键通过 `update_shortcuts` 稀疏补丁保存；显示“右 Alt / 未设置”；显示右 Alt 后端诊断；提供“恢复默认快捷键” |

## 2. 需求要点落实情况

- **默认值**：Windows 新配置 `shortcut=AltRight`、`shortcut2=""`、`extractShortcut=F6`、`extractShortcut2=""`，磁盘标准值 `AltRight`，UI 显示“右 Alt”。
- **空值**：显式 `""` 不注册、不触发、不回退默认；删除 F6 后不会自动恢复。
- **Backspace**：前端、后端保存入口、raw 迁移、运行时注册四层均归一化为空值；任何修饰键 + Backspace 同样删除。
- **规范化与冲突**：共享同一套 trim → Backspace → 别名 → 顺序 → 主键表示 → 合法性 → 冲突检查；`Ctrl+Shift+A` 与 `Shift+Ctrl+A` 等价；重复与 `AltRight`/Alt 前缀冲突在保存前拒绝。
- **启动注册**：先构造完整冲突图，冲突连通分量全部跳过，不依赖遍历顺序；单字段异常不 panic、不重置整个配置。
- **Windows AltRight**：Raw Input（`RIDEV_INPUTSINK`，不使用 `RIDEV_NOLEGACY`）首选，启用前用 `GetRegisteredRawInputDevices` 检查进程内已有键盘注册，冲突或失败时回退 `WH_KEYBOARD_LL`；同一时刻仅一个后端；只把右 Alt 转为应用事件。
- **generation 与捕获**：`registration_generation` 隔离重注册前后事件；事件带产生时 capture 快照，捕获期间产生的事件永久丢弃、不补发；capture 使用后端 token，旧 blur/cancel/end/commit 不影响新会话。
- **字段级保存**：`update_shortcuts` 稀疏补丁 → 校验 → 冲突图 → 注册切换 → 原子字段级 patch → generation 提交；任一步失败回滚到旧运行时与旧磁盘值，不触发 Local API 等无关副作用。
- **迁移**：`windowsShortcutSchemaVersion` 仅在 Windows 创建/推进；`schema<1` 必须先做完整检查，F4→AltRight 冲突时保留 F4，检查成功写盘后才置 1；写盘失败不假装完成，下次启动重试；`schema=1` 后用户改回 F4 不会再次迁移。
- **FutureSchemaMode**：`schema>1` 或未知类型不自动迁移、不自动写回、不降级；显式修改走无损字段级 patch，保留高版本 schema 与未知字段。

## 3. 诊断与可观测性（§19.2）

- 每次启动注册或保存后，把以下内容追加到 `<app_data_dir>/shortcut.log`（超过 512KB 自动重建）：
  - 所选右 Alt 后端（Raw Input / Hook）、schema 模式、PTT 模式；
  - 四个字段的原始值、规范化值、是否已注册、是否走原生后端、跳过原因；
  - 启动冲突图（冲突对及其类型）；
  - 注册失败的错误信息与迁移诊断（磁盘原始四字段、F4 是否因冲突保留）。
- `ShortcutStatus` 额外暴露 `nativeEventsSeen` / `nativeEventsDispatched` / `pttMode` / `capturing`，设置页可见，用于区分“后端未启动”“事件被捕获/重置丢弃”“已进入业务但无结果”。
- 隐私（§19.3）：日志只记录配置值与右 Alt 事件计数，不记录普通按键内容，不缓存/上传任何键盘数据。
- “恢复默认快捷键”（`reset_shortcuts`）可在不删除整个配置的前提下把四个字段重置为平台默认值。

## 4. 右 Alt 物理键识别

右 Alt 的虚拟键在不同驱动/键盘下可能是 `VK_MENU`（0x12）或扩展的 `VK_RMENU`（0xA5），仅匹配虚拟键会漏事件。因此两条后端都**优先用物理扫描码 `0x38` + `E0`/`EXTENDED` 标志**判定右 Alt，再以 `VK_MENU`+E0 / `VK_RMENU` 兜底；左 Alt 不会被误判。

## 5. 自动化验证

| 项目 | 命令 | 结果 |
|---|---|---|
| 前端构建 | `npm run build`（tsc + vite） | 通过 |
| Rust 纯逻辑 / 迁移测试 | 提取 `model.rs` + `shortcut_config.rs` 到独立 crate `cargo test` | 37 passed / 0 failed |
| Rust 全量类型检查（Windows 目标） | `cargo check --target x86_64-pc-windows-msvc` | 通过（仅 1 条与本需求无关的既有 `cleanup_screenshot_state` dead_code 警告） |
| Rust 测试编译（Windows 目标） | `cargo check --tests --target x86_64-pc-windows-msvc` | 通过 |

说明：本机为 Linux，缺少 `glib`/`webkit2gtk` 且无 root，无法执行原生 `cargo check`；因此使用 Windows 目标交叉类型检查。交叉检查所需的 C 依赖通过 `zig cc/ar` + `nasm` + 伪 `llvm-rc` 构建（仅用于 type-check，不落最终产物）。

## 6. 尚未完成的验收项（阻塞）

- **Windows 10/11 实机验收矩阵（需求 §23，T01–T40）未执行**：本环境无 Windows 实机，无法验证真实右/左 Alt、AltGr、Raw Input 与 Hook 的实际行为、后台/最小化、窗口重建、注入事件边界、系统占用等运行时行为。
- 因此 §22.5 中依赖实机的部分（Raw Input make/break 的真实解析、Hook 生命周期、后端切换、退出清理）仅通过代码与纯函数层面的保证，未做实机确认。
- 建议在 Windows 上按 §23 矩阵逐项执行并记录：系统版本与构建号、键盘型号/布局、Tauri 与 `tauri-plugin-global-shortcut` 版本、实际选择的 AltRight 后端、Raw Input 冲突检查结果、是否测试 AltGr。

## 7. 依赖与无关变更

- 新增依赖：无。
- `Cargo.toml` 仅新增 `windows-sys` 既有依赖的一个 feature。
- 未修改 `Cargo.lock` 与 `package-lock.json`。
- 未做全仓格式化、无关重构或与快捷键无关的业务改动。
