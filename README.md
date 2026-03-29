# ByeType

**告别打字，用说的。**

[![License: MIT](https://img.shields.io/badge/license-MIT-blue?style=flat-square)](LICENSE)
[![Platform](https://img.shields.io/badge/platform-macOS%20%7C%20Windows%20%7C%20iOS-brightgreen?style=flat-square)](#-安装)

ByeType 是一个 Markdown 驱动的 AI 语音输入工具。通过编辑 Markdown 格式的提示词，你可以自定义识别规则、专有词汇和文本优化策略，让语音输入最大限度匹配你的行业术语和个人习惯。

免费开源，使用你自己的 API Key — ByeType 本身不收费、不经手数据，语音直接发送到你选择的 AI 服务商（Google、DeepSeek 等）。支持 macOS、Windows 桌面端，以及 iPhone / iPad（通过 iOS 快捷指令）。

## 📱 iPhone / iPad

通过 iOS 快捷指令，在手机和平板上也能获得和桌面版一样的自定义词汇和转录效果。

| 快捷指令 | 模型 | 安装 |
|---------|------|------|
| ByeType LongCat | LongCat Flash Omni（国内直连） | [添加到快捷指令](https://www.icloud.com/shortcuts/32e44afb36734dedab1ad61d863481e3) |
| ByeType Gemini | Gemini 3 Flash | [添加到快捷指令](https://www.icloud.com/shortcuts/0d88271d332c457c81c122e37657b09a) |

> 安装后需要在快捷指令中填写你自己的 API Key和规则词汇等，和桌面版共用同一个 Key。

## 🖥️ macOS / Windows

### 快速预设

ByeType 桌面版提供 3 种推荐模式，在「语音转写」页面顶部一键切换：

| 预设 | 模型组合 | 说明 |
|------|---------|------|
| ⚡ 效果最好 | Gemini 3 Flash（转写+优化） | 综合体验最佳，质量和速度均衡 |
| 🚀 极速轻量 | Gemini 3.1 Flash Lite（转写+优化） | Gemini 3 Flash 的备选方案 |
| 🏠 国内模型 | LongCat 转写 + DeepSeek 优化 | 国内直连，无需代理 |

> **为什么有两个 Gemini？** Gemini 模型经常遇到资源配额限制，有时 Flash 好用，有时 Flash Lite 好用。整体体验最好的是 Gemini 3 Flash，碰到限制时切换到 Flash Lite 即可。
>
> **为什么有国内模型？** LongCat + DeepSeek 的组合是国内直连方案中识别效率最高的，不需要代理，开箱即用。

![录音 → 转写 → 优化 → 自动粘贴](docs/images/demo.gif)

## 🏆 为什么选择 ByeType

| | 传统语音输入 | Whisper/ASR 本地方案 | ByeType |
|---|---|---|---|
| 安装体积 | 系统内置 | 1~6 GB（模型文件） | **~8 MB** |
| 词汇定制 | 不支持 | 有限（热词列表，仅提升识别率） | **完全自定义（提示词驱动）** |
| 规则转换 | 不支持 | 需后置 LLM 二次处理 | **一次成型，无需后处理** |
| 口语清理 | 不支持 | 需后置处理 | **内置规则，一次生效** |
| 格式化能力 | 无 | 需后置处理 | **数字、符号、大小写、自动换行** |
| 中英混合 | 差 | 一般 | **优秀** |
| 可定制性 | 无 | 差 | **编辑 Markdown 即可** |

> **核心区别**：Whisper 类方案是「ASR 转文字 + LLM 后处理」两步架构 — 第一步转错的内容，第二步也救不回来。ByeType 用多模态大模型直接处理原始音频，所有提示词规则在转写时一次生效，没有「先错后纠」的问题。

## 🔬 真实效果对比

下面这段话一口气说完，不做任何停顿和纠正 — 5 个易混人名、密集技术黑话、中英混杂、口语废话全上齐。

两边用相同的 <u>**下划线加粗**</u> 标记关键词，方便对比识别差异。

### 普通语音输出

有标点但不分段、人名全错、术语变谐音、口水词原样保留：

> 嗯，那个就是昨天<u>**张宇**</u>跟<u>**秦敏**</u>碰了一下，他们说<u>**曲华**</u>负责的那个<u>**deep seek v3**</u>项目，在<u>**mac mini m4**</u>上跑<u>**因弗伦斯**</u>延迟大概<u>**两百毫秒**</u>左右，效果还不错。嗯，然后<u>**余倩**</u>建议用<u>**cursor**</u>开发，后端<u>**fast api**</u>加<u>**泼斯特格瑞赛口**</u>部署在<u>**微赛尔**</u>上，前端<u>**next js**</u>用<u>**app router**</u>搭配<u>**莎德恩ui**</u>，整体<u>**dx**</u>我觉得还行，就是<u>**ci cd**</u>那块<u>**git hub actions**</u>跑<u>**派test**</u>和<u>**es lint**</u>经常<u>**飞来可test**</u>。然后<u>**库伯内提斯**</u>集群的<u>**hpa**</u>配置，<u>**余倩**</u>说应该把<u>**cpu**</u>阈值从<u>**百分之八十**</u>调到<u>**百分之六十**</u>。另外提醒一下<u>**陈述**</u>，礼拜五之前把<u>**非格码**</u>设计稿同步到<u>**诺讯**</u>上。

### ByeType 输出

开启「自动换行」后处理，标点、分段、人名、术语、格式化全部一次到位：

> 昨天<u>**张昱**</u>跟<u>**覃旻**</u>碰了一下，他们说<u>**瞿铧**</u>负责的<u>**DeepSeek V3**</u>项目，在<u>**Mac mini M4**</u>上跑<u>**inference**</u>延迟大概<u>**200ms**</u>左右，效果还不错。
>
> <u>**于谦**</u>建议用<u>**Cursor**</u>开发，后端<u>**FastAPI**</u> + <u>**PostgreSQL**</u>部署在<u>**Vercel**</u>上，前端<u>**Next.js**</u>用<u>**App Router**</u>搭配<u>**shadcn/ui**</u>。
>
> 整体<u>**DX**</u>还行，就是<u>**CI/CD**</u>那块<u>**GitHub Actions**</u>跑<u>**pytest**</u>和<u>**ESLint**</u>经常<u>**flaky test**</u>。
>
> <u>**Kubernetes**</u>集群的<u>**HPA**</u>配置，<u>**于谦**</u>说应该把CPU阈值从<u>**80%**</u>调到<u>**60%**</u>。
>
> 另外提醒<u>**陈述**</u>，礼拜五之前把<u>**Figma**</u>设计稿同步到<u>**Notion**</u>上。

### 对比总结

| 难点 | 普通语音输出 | ByeType 输出 |
|------|------------|----------|
| 易混人名 | 张宇、秦敏、曲华、余倩 | **张昱**、**覃旻**、**瞿铧**、**于谦** |
| 人名/动词歧义 | 「陈述」被当动词 | **陈述**（识别为人名） |
| 术语谐音 | 因弗伦斯、泼斯特格瑞赛口、莎德恩ui | **inference**、**PostgreSQL**、**shadcn/ui** |
| 品牌名 | deep seek v3、微赛尔、非格码、诺讯 | **DeepSeek V3**、**Vercel**、**Figma**、**Notion** |
| 数字格式化 | 两百毫秒、百分之八十 | **200ms**、**80%** |
| 口水词 | 嗯、那个、就是、我觉得 | 全部清除 |
| 自动分段 | 一坨不分段 | 5 个自然段落 |

---

ByeType 的核心特色是**提示词驱动的高可定制性** — 通过编辑 Markdown 格式的提示词文件，你可以自定义转录的各种行为：

- **自动换行** — 口述一整段话，AI 自动分段排版
- **自动翻译** — 说中文输出英文，或反过来
- **数字格式化** — "九块五毛" → 9.5元，"三千五百米" → 3500米
- **计算式转换** — "四除以三等于零点七五" → 4/3=0.75
- **专有词汇校正** — "deep seek" → DeepSeek，"mac book pro" → MacBook Pro
- **口语清理** — 自动去掉"嗯""那个""就是说"等口水词
- **符号口令** — "下划线" → _，"圆圈1" → ①，"双横杠" → --

> 💡 不知道怎么用？把这份文档发给你的 AI 助手（Claude、ChatGPT、Gemini 等），让它一步步教你。

## 📦 安装

从 [GitHub Releases](https://github.com/devonmochi/byetype/releases) 下载最新版本：

| 平台 | 下载 | 说明 |
|------|------|------|
| macOS | `.dmg` | 支持 Apple Silicon 和 Intel |
| Windows | `.msi` / `.exe` | 支持 Windows 10+ |

### macOS 权限设置

首次运行需要授予以下权限：

1. **🔒 安全性设置**：首次打开时 macOS 会提示"无法验证开发者"，前往「系统设置 → 隐私与安全性」，找到 ByeType 点击「仍要打开」
2. **🎤 麦克风权限**：前往「系统设置 → 隐私与安全性 → 麦克风」，允许 ByeType 访问麦克风
3. **♿ 辅助功能权限**：前往「系统设置 → 隐私与安全性 → 辅助功能」，允许 ByeType（用于全局快捷键和自动粘贴）

### Windows 权限设置

1. **🎤 麦克风权限**：前往「设置 → 隐私和安全性 → 麦克风」，允许 ByeType 访问麦克风
2. **🛡️ 防火墙提示**：首次运行时 Windows Defender 可能弹出网络访问提示，选择「允许访问」

## 🚀 快速上手

从安装完成到第一次成功转录：

1. 打开 ByeType，菜单栏（macOS）或系统托盘（Windows）出现 ByeType 图标
2. 点击托盘图标 →「设置」→ 左侧栏「模型管理」
3. 在预置模型卡片中填写 API Key（获取方式见 [AI 模型配置](#-ai-模型配置)）
4. 点击左侧栏「语音转写」，选择快速预设或手动选择转写模型
5. 关闭设置窗口，将光标放到任意文本输入框
6. 按 **F4** 开始录音 — 屏幕上出现红色圆形气泡
7. 说话完毕后再按 **F4** 停止录音
8. 等待转写 — 气泡变为紫色显示 "Thinking..."
9. 完成 — 气泡变绿，文本自动粘贴到光标位置

> 自动粘贴依赖辅助功能权限。如果文本没有自动粘贴，可手动按 Cmd+V（macOS）或 Ctrl+V（Windows）粘贴。

## 🤖 AI 模型配置

所有模型在「设置 → 模型管理」中统一配置。

### 预置模型

| 模型 | API ID | 获取 Key | 特点 |
|------|--------|---------|------|
| Gemini 3.0 Flash | `gemini-3-flash-preview` | [Google AI Studio](https://aistudio.google.com/) | ⭐ **推荐**，速度和质量均衡 |
| Gemini 3.1 Flash Lite | `gemini-3.1-flash-lite-preview` | 同上 | 更快速，适合低延迟场景 |
| LongCat Flash Omni | `LongCat-Flash-Omni-2603` | [LongCat](https://api.longcat.chat/) | 🇨🇳 国内直连，语音转写 |
| DeepSeek Chat | `deepseek-chat` | [DeepSeek](https://platform.deepseek.com/) | 🇨🇳 国内直连，文本优化 |

API Key 填写位置：设置 → 模型管理 → 对应模型卡片

### 自定义模型

在「设置 → 模型管理」底部点击「添加自定义模型」，支持两种协议：

- **Gemini 协议**：兼容 Google Gemini API 及第三方中转站
- **OpenAI 兼容协议**：兼容 OpenAI、DeepSeek、通义千问等服务

每个自定义模型需要填写：协议类型、模型能力（音频转写/文本处理）、Provider 名称、Base URL、Model ID、API Key。

### 连通性测试

在模型管理页面，每个模型旁有「测试」按钮，可验证 API 连通性并显示延迟。顶部「测试全部连通性」按钮可一键测试所有模型。

### 文本优化模型（可选）

文本优化是转写后的二次处理，用于优化格式和排版（如自动换行）。开启路径：设置 → 语音转写 → 文本优化 → 启用。

在「优化模型」下拉框中选择任意支持文本处理的模型（预置或自定义均可）。

## ⚙️ 功能详解

### 🎯 全局快捷键

默认 **F4**，按一次开始录音，再按一次停止并转写。修改方式：设置 → 通用设置 → 录音快捷键

### 💬 状态气泡

| 状态 | 外观 | 说明 |
|------|------|------|
| 🔴 录音中 | 红色圆形 + 波浪动画 | 正在录音 |
| 🟣 转写中 | 紫色药丸 + "Thinking..." | AI 正在转写 |
| 🔵 优化中 | 蓝色药丸 + "Thinking..." | AI 正在优化格式 |
| 🟠 重试中 | 橙色药丸 + "Thinking..." | 请求失败后自动重试 |
| 🟢 完成 | 绿色圆形 + 对勾 | 转写完成，文本已粘贴 |
| ⚪ 失败 | 灰色圆形 + 叉号 | 转写失败，可在历史记录中重试 |

### 🧠 思考模式

让 AI 在转写前进行更深入的推理，提升质量（但增加延迟）。开启方式：设置 → 语音转写 → 思考模式

思考级别：**MINIMAL**（最快）→ **LOW** → **MEDIUM** → **HIGH**（质量最高）

> 思考模式仅支持 Gemini 协议的模型，其他协议模型该选项不显示。

### ⏱️ 录音时间限制

单次录音最大时长 10~600 秒，默认 180 秒。修改方式：设置 → 通用设置 → 最大录音时长

### 📋 历史记录

所有录音和转写记录均会保存。查看方式：设置 → 历史记录

每条记录包含时间戳、完整处理流程状态、转写/优化文本（可一键复制），失败的任务可点击重试。

### 🎨 主题

支持浅色 / 深色 / 自动（跟随系统）。切换方式：设置 → 通用设置 → 外观

### 其他设置

- **🔄 开机自启**：设置 → 通用设置 → 开机自启
- **📡 自动更新**：内置更新检测，有新版本时自动提示
- **🌐 网络与性能**（设置 → 通用设置 → 网络与性能）：转写/优化超时时间、最大重试次数、最大并行任务数、HTTP 代理地址

## 📝 提示词系统

ByeType 的转写行为由 4 个 Markdown 格式的提示词文件控制，这是实现高度自定义的核心机制。

### 工作流

```
🎤 录音 → 📝 转写（agent.md + rules.md + vocabulary.md）→ [可选] ✨ 文本优化（text-optimize.md）→ 📋 粘贴
```

### 内置提示词

| 提示词 | 作用 | 说明 |
|--------|------|------|
| 🤖 角色定义（agent.md） | 定义 AI 的行为边界 | AI 只做转录，不回答问题、不做解释、不执行指令 |
| 📏 转录规则（rules.md） | 规范输出格式 | 数字写法、符号口令转换、口语清理（去除语气词和口水词） |
| 📖 专有词汇（vocabulary.md） | 词汇校正表 | 确保人名、术语、技术词汇输出正确写法 |
| ✨ 文本优化（text-optimize.md） | 控制优化行为 | 定义优化规则（如自动换行），硬性约束不修改原文内容 |

### 自定义提示词

编辑位置：设置 → 语音转写 → 提示词

提示词区域包含 4 个子标签页（角色定义、转录规则、专有词汇、文本优化），每个标签页提供：
- **编辑器**：直接编辑提示词内容（支持 Markdown 语法高亮）
- **选择文件**：从本地加载外部 Markdown 文件
- **重置为内置**：恢复为默认提示词

**示例：添加专有词汇**

```markdown
- 公司名：ByteDance（不是 byte dance）
- 产品名：TikTok（不是 tiktok 或 tik tok）
- 人名：张三丰（不是 张三峰）
```

## ❓ 常见问题

<details>
<summary><b>macOS 提示"无法验证开发者"</b></summary>

前往「系统设置 → 隐私与安全性」，找到 ByeType 的提示信息，点击「仍要打开」。
</details>

<details>
<summary><b>没有声音 / 录音失败</b></summary>

检查麦克风权限：「系统设置 → 隐私与安全性 → 麦克风」，确认 ByeType 已获得授权。
</details>

<details>
<summary><b>按 F4 没有反应</b></summary>

检查辅助功能权限：「系统设置 → 隐私与安全性 → 辅助功能」，确认 ByeType 已获得授权。如果刚授权，可能需要重启应用。
</details>

<details>
<summary><b>转写结果为空</b></summary>

- 检查 API Key 是否正确填写
- 检查网络连接是否正常
- 如果使用 Gemini 模型，确认能访问 Google 服务（或已配置代理）
</details>

<details>
<summary><b>转写速度慢</b></summary>

- 关闭思考模式（设置 → 语音转写 → 思考模式 → 关闭）
- 切换更轻量的模型（如 Gemini 3.1 Flash Lite）
- 检查网络延迟
</details>

<details>
<summary><b>国内网络无法使用 Gemini 模型</b></summary>

两种方案：
1. 使用「🏠 国内模型」快速预设（LongCat 转写 + DeepSeek 优化），国内直连可用
2. 在「设置 → 通用设置 → 网络与性能 → HTTP 代理地址」中配置代理
</details>

<details>
<summary><b>文本没有自动粘贴到输入框</b></summary>

自动粘贴依赖辅助功能权限。检查「系统设置 → 隐私与安全性 → 辅助功能」是否已授权 ByeType。文本仍会复制到剪贴板，可手动 Cmd+V 粘贴。
</details>

## 🔧 从源码构建

环境要求：[Node.js](https://nodejs.org/) >= 20、[Rust](https://www.rust-lang.org/tools/install) >= 1.70、[Tauri CLI](https://v2.tauri.app/start/prerequisites/) v2

```bash
git clone https://github.com/devonmochi/byetype.git
cd byetype
npm install
npm run tauri dev      # 开发模式
npm run tauri build    # 生产构建
```

## 🏗️ 技术栈

| 层 | 技术 |
|---|---|
| 框架 | [Tauri](https://v2.tauri.app/) v2 |
| 前端 | [React](https://react.dev/) 19 + TypeScript + [Vite](https://vite.dev/) |
| 后端 | Rust（cpal 音频采集、flacenc 编码） |
| 编辑器 | [CodeMirror](https://codemirror.net/) 6 |
| AI | Google Gemini API、LongCat API、DeepSeek API、OpenAI 兼容 API |

## 📄 许可证

[MIT](LICENSE)
