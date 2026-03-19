# 设置界面菜单重构设计

## 背景

ByeType 设置界面当前有 6 个侧边栏标签页：历史记录、通用设置、转写设置、文本优化、提示词、高级设置。存在以下问题：

- **命名不统一**：「转写设置」用「XX设置」，「文本优化」用动作描述，风格混杂
- **分类不合理**：语音转写和文本优化属于同一条处理流水线（录音→转写→优化），不应分开；通用设置和高级设置都是应用级配置，没必要拆成两个
- **提示词归属不清**：独立的「提示词」标签页里混合了转写类和优化类提示词，跟前面的功能分类逻辑矛盾
- **标签过多**：6 个标签页对于一个轻量工具来说偏多，增加认知负担

## 设计目标

- 菜单数量精简，降低认知负担
- 命名直观，看名字就知道里面是什么
- 分类符合功能逻辑
- 高频使用的标签排在前面

## 设计约束

- **配置结构不变**：`AppConfig` 类型（`src/core/types.ts`）及 Rust 后端的序列化/反序列化保持不变，四个顶级字段 `general`、`transcribe`、`optimize`、`advanced` 原样保留。本次只做 UI 层合并，不改底层数据结构，确保已有用户配置文件兼容。
- **标签 id 不变**：合并后的标签 `id` 保持原值（`history`、`transcribe`、`general`），仅更新 `label` 显示文本。避免持久化逻辑的兼容问题。

## 菜单结构：6 → 3

| 顺序 | 标签名 | 说明 |
|------|--------|------|
| 1 | 历史记录 | 查看过往录音/转写/优化记录（只读视图） |
| 2 | 语音转写 | 合并原「转写设置」+「文本优化」+「提示词」 |
| 3 | 通用设置 | 合并原「通用设置」+「高级设置」 |

排序依据：按使用频率从高到低排列。历史记录最常用（查看转写结果），语音转写次之（调模型和提示词），通用设置最少改动。

## 「历史记录」标签页

内容不变，保持现有只读视图。

## 「语音转写」标签页

长页面滚动，用标题分隔不同区域。包含三个区域：

### 区域一：转写模型

保留现有的三个 SettingGroup 分组，归在同一个区域标题下：

**模型**
- 转写模型选择（Gemini 3.0 Flash / Gemini 3.1 Flash Lite / Qwen3 Omni Flash）

**API 密钥**
- Google Gemini API Key
- 阿里云 Qwen API Key

**思考模式**
- 启用思考开关
- Thinking Level 选择（MINIMAL / LOW / MEDIUM / HIGH）

### 区域二：文本优化

- 启用文本优化开关
- 优化引擎选择（OpenAI 兼容 / Gemini）
- OpenAI 兼容配置：Provider 名称、Base URL、Model、API Key
- Gemini 配置：模型选择、启用思考、Thinking Level

### 区域三：提示词

保留原有的子标签切换方式，包含 4 个提示词：
- 角色定义
- 转录规则
- 专有词汇
- 文本优化

每个提示词支持：文件路径显示、选择自定义文件、重置为内置、Markdown 编辑器。

**迁移策略**：将 PromptsTab 整体作为独立子组件嵌入（保持内部状态独立），不打散与 TranscribeTab 合并。原因：PromptsTab 包含 CodeMirror 编辑器的完整生命周期管理、防抖保存、文件系统操作等复杂逻辑，拆散会引入不必要的风险。用 `React.memo` 包裹，隔离父组件 config 变更对编辑器的影响（避免修改转写模型等配置时触发编辑器重载）。

**编辑器高度**：在滚动布局中，编辑器不再能用 flex 填充。给编辑器区域设置固定高度 `400px`。

**Gemini 复用提示**：合并到同一页面后，文本优化 Gemini 配置中「复用转写设置中的 API Key」的提示语保持不变，因为两个区域在滚动页面中有视觉距离，提示仍有用。

## 「通用设置」标签页

长页面滚动，用标题分隔不同区域。包含三个区域：

### 区域一：外观

- 主题选择（浅色 / 深色 / 自动）

### 区域二：通用

- 录音快捷键
- 开机自启

### 区域三：网络与性能

- 转写超时时间（秒）
- 文本优化超时时间（秒）
- 最大重试次数
- 最大并行任务数
- HTTP 代理地址

## 涉及的文件变更

### 修改
- `src/views/settings/App.tsx` — TABS 数组从 6 项改为 3 项（保留 id 不变），移除 OptimizeTab 和 AdvancedTab 的 import 和渲染
- `src/views/settings/tabs/TranscribeTab.tsx` — 整合 OptimizeTab 的配置 UI，嵌入 PromptsTab 作为子组件
- `src/views/settings/tabs/GeneralTab.tsx` — 整合 AdvancedTab 的配置项（超时、重试、并行、代理）
- `src/views/settings/tabs/PromptsTab.tsx` — 改造为可嵌入的子组件，用 `React.memo` 包裹，设置固定编辑器高度 400px

### 删除
- `src/views/settings/tabs/OptimizeTab.tsx` — 内容合并到 TranscribeTab
- `src/views/settings/tabs/AdvancedTab.tsx` — 内容合并到 GeneralTab

### 不变
- `src/views/settings/tabs/HistoryTab.tsx` — 无变更
- `src/views/settings/components/` — SettingGroup、SettingRow、Toggle 等共用组件无变更
- `src/core/types.ts` — AppConfig 类型结构不变
- Rust 后端配置相关代码 — 不变

## 默认标签页

打开设置窗口默认显示「历史记录」，保持不变。
