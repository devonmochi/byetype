# ByeType

**告别打字，用说的。**

[![License: MIT](https://img.shields.io/badge/license-MIT-blue?style=flat-square)](LICENSE)
[![Platform](https://img.shields.io/badge/platform-macOS%20%7C%20Windows%20%7C%20iOS-brightgreen?style=flat-square)](https://github.com/devonmochi/byetype/releases)

ByeType 是一个 Markdown 驱动的 AI 语音输入工具。通过编辑 Markdown 格式的提示词，你可以自定义识别规则、专有词汇和文本优化策略，让语音输入最大限度匹配你的行业术语和个人习惯。

此外，ByeType 还内置了 **AI 图像文字提取**功能（同样由 Markdown 提示词驱动），完美解决终端中 AI 输出的代码因行号、分屏、换行被切碎而无法直接复制使用的问题。

免费开源，使用你自己的 API Key — ByeType 本身不收费、不经手数据，语音和截图直接发送到你选择的 AI 服务商（Google、阿里云百炼等）。支持 macOS、Windows 桌面端，以及 iPhone / iPad（通过 iOS 快捷指令）。

## 📱 iPhone / iPad

通过 iOS 快捷指令，在手机和平板上也能获得和桌面版一样的自定义词汇和转录效果。

| 快捷指令 | 模型 | 安装 |
|---------|------|------|
| ByeType LongCat | LongCat Flash Omni（国内直连） | [添加到快捷指令](https://www.icloud.com/shortcuts/32e44afb36734dedab1ad61d863481e3) |
| ByeType Gemini | Gemini 3 Flash | [添加到快捷指令](https://www.icloud.com/shortcuts/0d88271d332c457c81c122e37657b09a) |

> 安装后需要在快捷指令中填写你自己的 API Key和规则词汇等，和桌面版共用同一个 Key。

## 🖥️ macOS / Windows

![录音 → 转写 → 优化 → 自动粘贴](docs/images/demo.gif)

## 🏆 为什么选择 ByeType

| | ByeType | 传统语音输入 | Whisper/ASR 本地方案 |
|---|---|---|---|
| 安装体积 | **~8 MB** | 系统内置 | 1~6 GB（模型文件） |
| 词汇定制 | **完全自定义（提示词驱动）** | 不支持 | 有限（热词列表，仅提升识别率） |
| 规则转换 | **一次成型，无需后处理** | 不支持 | 需后置 LLM 二次处理 |
| 口语清理 | **内置规则，一次生效** | 不支持 | 需后置处理 |
| 格式化能力 | **数字、符号、大小写、自动换行** | 无 | 需后置处理 |
| 中英混合 | **优秀** | 差 | 一般 |
| 可定制性 | **编辑 Markdown 即可** | 无 | 差 |
| 多场景切换 | **日常口语、正式书写、翻译等快速切换** | 不支持 | 不支持 |

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

## 📸 图像文字提取 — 不只是 OCR

按 **F6** 截图选区，AI 自动识别文字并复制到剪贴板。同样由 Markdown 提示词驱动（`text-extract.md`），可自定义识别行为。ByeType 用多模态大模型理解截图的视觉布局，能做到传统 OCR 做不到的事：

### 示例 1：智能换行修复

终端、浏览器、PDF 阅读器里的文字经常因窗口宽度被硬截断。传统 OCR 原样照搬断行，ByeType 理解语义后自动合并为完整段落。

#### 传统 OCR 输出

逐行照搬，保留所有因窗口宽度产生的硬换行：

> 人工智能(AI)正在迅速发展，它已经开始<br>改变我们的生活方式和工作方式。从智能<br>手机助手到自动驾驶汽车，AI技术正在<br>各个领域展现其潜力。

#### ByeType 输出

理解语义，自动合并断行为完整段落：

> 人工智能(AI)正在迅速发展，它已经开始改变我们的生活方式和工作方式。从智能手机助手到自动驾驶汽车，AI技术正在各个领域展现其潜力。

### 示例 2：终端代码智能还原

在 Claude Code、终端、IDE 里截图代码时，行号、提示符、分屏边界会把代码切得支离破碎。ByeType 能识别哪些是代码、哪些是装饰，还原出干净可用的代码块。

#### 传统 OCR 输出

行号、管道符原样输出，因窗口宽度导致的断行也照搬：

```
  1 │ fn main() {
  2 │     let items = vec!["hel
  3 │ lo", "world"];
  4 │     for item in &items
  5 │ {
  6 │         println!("{}",
  7 │ item);
  8 │     }
  9 │ }
```

#### ByeType 输出

去除行号装饰，修复断行，自动标注语言，输出可直接使用的完整代码：

```rust
fn main() {
    let items = vec!["hello", "world"];
    for item in &items {
        println!("{}", item);
    }
}
```

## 🤖 AI 模型配置

所有模型在「设置 → 模型管理」中统一配置。

### 预置模型

| 模型 | API ID | 获取 Key | 特点 |
|------|--------|---------|------|
| Qwen 3.5 Omni Plus | `qwen3.5-omni-plus` | [阿里云百炼](https://bailian.console.aliyun.com/) | ⭐ **推荐**，国内直连，效果好 |
| Qwen 3.5 Omni Flash | `qwen3.5-omni-flash` | 同上 | 国内直连，速度更快 |
| LongCat Flash Omni | `LongCat-Flash-Omni-2603` | [LongCat](https://platform.longcat.chat/) | 国内直连，Qwen 的替代方案 |
| MiMo v2.5 | `mimo-v2.5` | [小米 MiMo](https://api.xiaomimimo.com/) | 国内直连，Qwen 的替代方案 |
| Gemini 3.0 Flash | `gemini-3-flash-preview` | [Google AI Studio](https://aistudio.google.com/) | 速度和质量均衡，需代理 |
| Gemini 3.1 Flash Lite | `gemini-3.1-flash-lite-preview` | 同上 | 更快速，适合低延迟场景，需代理 |
| DeepSeek V4 Flash | `deepseek-v4-flash` | [DeepSeek](https://platform.deepseek.com/) | 仅文本优化，速度快、成本低 |
| DeepSeek V4 Pro | `deepseek-v4-pro` | 同上 | 仅文本优化，质量更高 |

> **OpenRouter 中转**：如果无法直接访问 Gemini，可通过 [OpenRouter](https://openrouter.ai/) 中转使用以下模型，无需代理：
> - `google/gemini-3-flash-preview`
> - `google/gemini-3.1-flash-lite-preview`

## 📝 提示词与模板系统

ByeType 的转写行为由提示词文件和输出风格共同控制，这是实现高度自定义的核心机制。

### 工作流

**语音转写：**
```
🎤 录音 → 📝 转写（agent.md + rules.md + vocabulary.md）→ [可选] ✨ 模板处理（绑定的模板 prompt）→ 📋 粘贴
```

**图像识别：**
```
📸 截图 → 🔍 模板处理（绑定的模板 prompt）→ 📋 复制到剪贴板
```

### 转写提示词

控制语音转写第一步（音频→文字）的行为，编辑位置：设置 → 转写提示词

| 提示词 | 作用 | 说明 |
|--------|------|------|
| 🤖 角色定义（agent.md） | 定义 AI 的行为边界 | AI 只做转录，不回答问题、不做解释、不执行指令 |
| 📏 转录规则（rules.md） | 规范输出格式 | 数字写法、符号口令转换、口语清理（去除语气词和口水词） |
| 📖 专有词汇（vocabulary.md） | 词汇校正表 | 确保人名、术语、技术词汇输出正确写法 |

### 处理模板

模板定义转写后的二次处理（语音侧）或截图后的 AI 处理（图像侧）。每个快捷键可绑定一个模板。

**语音模板**（设置 → 转写提示词 → 文本优化提示词）：

| 模板 | 说明 |
|------|------|
| 自动换行 | 转写后自动分段排版，不修改原文内容 |
| 翻译 | 将转写文本翻译为英文 |
| 自定义 | 用户自定义 prompt |
| 无 | 跳过二次处理，转写结果直接粘贴 |

**图像模板**（设置 → 图像识别提示词）：

| 模板 | 说明 |
|------|------|
| 文字识别 | 识别截图中的文字，还原布局和格式 |
| 翻译 | 识别截图文字并翻译为中文 |
| 自定义 | 用户自定义 prompt |

所有模板的名称和 prompt 均可编辑，也可添加新模板。内置模板支持「重置为内置」恢复默认。

### 自定义提示词

每个提示词/模板编辑器提供：
- **编辑器**：直接编辑内容（支持 Markdown 语法高亮）
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
1. 在「设置 → 语音转写」中选择 Qwen 3.5 Omni 等国内直连模型，无需代理
2. 在「设置 → 通用设置 → 网络与性能 → HTTP 代理地址」中配置代理后使用 Gemini
</details>

<details>
<summary><b>文本没有自动粘贴到输入框</b></summary>

自动粘贴依赖辅助功能权限。检查「系统设置 → 隐私与安全性 → 辅助功能」是否已授权 ByeType。文本仍会复制到剪贴板，可手动 Cmd+V 粘贴。
</details>

## 🏗️ 技术栈

| 层 | 技术 |
|---|---|
| 框架 | [Tauri](https://v2.tauri.app/) v2 |
| 前端 | [React](https://react.dev/) 19 + TypeScript + [Vite](https://vite.dev/) |
| 后端 | Rust（cpal 音频采集、flacenc 编码） |
| 编辑器 | [CodeMirror](https://codemirror.net/) 6 |
| AI | Google Gemini API、阿里云百炼 DashScope API、OpenAI 兼容 API |

## 📄 许可证

[MIT](LICENSE)

---

如果这个项目对你有帮助，欢迎点一个 Star。

欢迎提 Issue 或直接发 PR。感谢 [Linux.do](https://linux.do/) 社区推动。
