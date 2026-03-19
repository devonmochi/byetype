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

- 转写模型选择（Gemini 3.0 Flash / Gemini 3.1 Flash Lite / Qwen3 Omni Flash）
- Google Gemini API Key
- 阿里云 Qwen API Key
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
- `src/views/settings/App.tsx` — 菜单定义从 6 项改为 3 项，移除 PromptsTab 和 AdvancedTab 的引用
- `src/views/settings/tabs/TranscribeTab.tsx` — 重命名为语音转写，整合文本优化配置和提示词编辑
- `src/views/settings/tabs/GeneralTab.tsx` — 整合高级设置的配置项

### 删除
- `src/views/settings/tabs/OptimizeTab.tsx` — 内容合并到 TranscribeTab
- `src/views/settings/tabs/AdvancedTab.tsx` — 内容合并到 GeneralTab
- `src/views/settings/tabs/PromptsTab.tsx` — 内容合并到 TranscribeTab

### 不变
- `src/views/settings/tabs/HistoryTab.tsx` — 无变更

## 默认标签页

打开设置窗口默认显示「历史记录」，保持不变。
