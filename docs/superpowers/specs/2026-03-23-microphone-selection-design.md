# 麦克风选择功能设计

## 概述

为 ByeType 增加麦克风输入设备选择功能，让用户可以在设置界面选择和预览输入设备，而非始终使用系统默认设备。

## 需求

- 设置界面下拉菜单选择输入设备
- 实时音量指示器预览麦克风收音
- 包含"系统默认"选项，跟随系统音频设置
- 设备断开时静默回退到系统默认设备
- UI 放置在"通用"标签页

## 方案：按需枚举

打开设置页面时枚举设备列表，录音启动时按名称查找设备。低频操作不需要实时设备监听，刷新按钮覆盖热插拔场景。

## 数据模型与配置

### Rust 配置

`GeneralConfig` 新增 `microphone` 字段：

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GeneralConfig {
    pub shortcut: String,
    pub launch_at_login: bool,
    pub theme: String,
    #[serde(default = "default_max_recording_seconds")]
    pub max_recording_seconds: u32,
    #[serde(default = "default_microphone")]
    pub microphone: String,  // "system-default" 或具体设备名
}

fn default_microphone() -> String {
    "system-default".to_string()
}
```

同步更新 `impl Default for AppConfig` 中 `GeneralConfig` 的默认值：

```rust
general: GeneralConfig {
    shortcut: "F4".to_string(),
    launch_at_login: false,
    theme: "system".to_string(),
    max_recording_seconds: 180,
    microphone: "system-default".to_string(),  // 新增
},
```

### TypeScript 配置

在 `src/core/types.ts` 中新增 `AudioDevice` 接口和 `GeneralConfig` 字段：

```typescript
interface AudioDevice {
    name: string;
    isDefault: boolean;
}
```

`GeneralConfig` 新增字段：

```typescript
microphone: string;  // "system-default" 或具体设备名
```

默认值 `"system-default"`，通过 `#[serde(default)]` 确保旧配置文件向后兼容。

## Rust 后端新增命令

### 数据结构

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioDevice {
    pub name: String,
    pub is_default: bool,
}
```

### 新增 `VolumeMonitor` 托管状态

音量监控需要跨命令共享状态，新增 `VolumeMonitor` 结构体并通过 `app.manage()` 注册（与现有 `ConfigManager`、`AudioRecorder` 方式一致）：

```rust
pub struct VolumeMonitor {
    running: Arc<AtomicBool>,
}
```

在 `lib.rs` 的 `.manage()` 链中注册此状态。

### 1. `list_input_devices`

枚举可用输入设备。

```rust
#[tauri::command]
fn list_input_devices() -> Result<Vec<AudioDevice>, String>
```

- 返回列表首项固定为 `{ name: "system-default", is_default: false }`
- 其余为 cpal 枚举到的输入设备，标记 `is_default` 表示系统默认设备

### 2. `start_volume_monitor`

启动音量监控。**幂等设计**：如果已有监控在运行，先自动停止旧监控再启动新的，无需依赖前端保证调用顺序。

```rust
#[tauri::command]
fn start_volume_monitor(
    device_name: String,
    app: AppHandle,
    monitor: State<VolumeMonitor>
) -> Result<(), String>
```

- 根据 `device_name` 打开对应设备的输入流
- `"system-default"` 使用 `host.default_input_device()`
- 以约 10Hz 频率计算 RMS 音量值（0.0 ~ 1.0）
- 通过 `app.emit("volume-level", level)` 推送到前端
- 通过 `VolumeMonitor.running`（`Arc<AtomicBool>`）控制停止

### 3. `stop_volume_monitor`

停止音量监控。

```rust
#[tauri::command]
fn stop_volume_monitor(monitor: State<VolumeMonitor>) -> Result<(), String>
```

设置 `AtomicBool` 标志，终止监控线程和音频流。

### 录音逻辑改造

`recorder.rs` 中 `AudioRecorder::start()` 方法改为接受设备名称参数：

```rust
pub fn start(&self, device_name: &str) -> Result<(), String>
```

- `"system-default"` -> `host.default_input_device()`
- 其他值 -> 遍历 `host.input_devices()` 按名称匹配
- 匹配失败 -> 静默回退到默认设备

`shortcut.rs` 中调用 `recorder.start()` 的地方需同步修改：从 `ConfigManager` 读取 `microphone` 字段，传入 `recorder.start(device_name)`。

### 设备查找辅助函数

抽取公共的设备查找逻辑为辅助函数，供 `start_volume_monitor` 和 `AudioRecorder::start()` 共用：

```rust
fn find_input_device(device_name: &str) -> Option<cpal::Device>
```

放在 `audio/mod.rs` 中。

## 前端 UI

### 位置

`GeneralTab.tsx` 中，作为新的 `SettingGroup`。

### 组件结构

```
SettingGroup: "麦克风"
  SettingRow: "输入设备"
    <select> 下拉菜单（设备列表）
    刷新按钮（重新枚举设备）
  SettingRow: "音量预览"
    音量条（水平进度条，实时显示 RMS 音量）
```

### 交互流程

1. 进入通用标签页 -> 调用 `list_input_devices` 获取设备列表，填充下拉菜单
2. 下拉菜单首项"系统默认"，其余为具体设备名，当前配置值高亮选中
3. 切换设备 -> 写入配置 + 调用 `start_volume_monitor`（后端幂等，自动停止旧监控）
4. 刷新按钮 -> 重新调用 `list_input_devices`，若当前选中设备不在新列表中，自动切换到"系统默认"
5. 音量条 -> 监听 `volume-level` 事件，绿色填充，实时更新
6. 离开标签页/关闭设置窗口 -> 调用 `stop_volume_monitor`

### 音量条样式

水平条形：灰色背景，绿色填充，高度约 6-8px，圆角。跟随项目浅色/深色主题。

### 前端 API 封装

在 `src/lib/tauri-api.ts` 中新增 3 个函数封装 Tauri invoke 调用：

- `listInputDevices(): Promise<AudioDevice[]>`
- `startVolumeMonitor(deviceName: string): Promise<void>`
- `stopVolumeMonitor(): Promise<void>`

## 错误处理

| 场景 | 处理方式 |
|------|----------|
| 配置中的设备名在设备列表中找不到 | 静默回退到系统默认设备 |
| 系统无任何输入设备 | `list_input_devices` 返回仅含 "system-default" 的列表；录音时报错 |
| 音量监控启动失败 | 前端音量条显示为空/灰色，不阻塞其他操作 |
| 录音过程中切换设备 | 本次录音不受影响，下次录音生效 |
| macOS 麦克风权限被拒绝 | 音量条灰显，设备列表仍可选择，权限问题不阻塞配置保存 |
| 应用退出时监控仍在运行 | 在应用退出钩子中调用 `stop_volume_monitor` 清理资源 |

## 数据流

```
设置页面打开
  -> invoke("list_input_devices")
  -> 填充下拉菜单 + 选中当前配置值
  -> invoke("start_volume_monitor", { deviceName })
  -> listen("volume-level") -> 更新音量条

用户切换设备
  -> 更新配置（写入 config.json）
  -> invoke("start_volume_monitor", { newDeviceName })  // 幂等，自动停止旧监控

用户触发录音（快捷键）
  -> shortcut.rs 读取 config.general.microphone
  -> 传入 recorder.start(device_name)
  -> 按名称查找设备 / 使用默认设备
  -> 开始录音

离开设置页 / 关闭窗口
  -> invoke("stop_volume_monitor")

应用退出
  -> 退出钩子中调用 stop_volume_monitor 清理
```

## 已知限制

- **设备名称唯一性**：使用设备名称（String）作为标识符。cpal 不保证名称唯一，两个同型号设备可能返回相同名称。在实际使用中（macOS/Windows）此情况极为罕见，暂不处理。
- **设备热插拔**：不实时监听设备变更，需用户手动点击刷新按钮更新列表。

## 涉及文件

- `src-tauri/src/config/types.rs` — 新增 microphone 字段 + 更新 Default impl
- `src-tauri/src/audio/mod.rs` — 新增 `find_input_device` 辅助函数
- `src-tauri/src/audio/recorder.rs` — `start()` 方法接受设备名称参数
- `src-tauri/src/commands.rs` — 新增 3 个 Tauri 命令 + VolumeMonitor 结构体
- `src-tauri/src/shortcut.rs` — 传递 microphone 配置到 recorder.start()
- `src-tauri/src/lib.rs` — 注册新命令 + manage(VolumeMonitor) + 退出钩子
- `src/core/types.ts` — TypeScript 类型同步
- `src/lib/tauri-api.ts` — 新增 3 个 API 封装函数
- `src/views/settings/tabs/GeneralTab.tsx` — 麦克风设置 UI
- `src/views/settings/index.css` — 音量条样式
