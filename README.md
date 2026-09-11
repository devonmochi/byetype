# ByeType

**告别打字，用说的。**

[![License: MIT](https://img.shields.io/badge/license-MIT-blue?style=flat-square)](LICENSE)
[![Platform](https://img.shields.io/badge/platform-macOS%20%7C%20Windows%20%7C%20iOS-brightgreen?style=flat-square)](https://github.com/devonmochi/byetype/releases)

ByeType 是一个 Markdown 驱动的 AI 语音输入工具。按 F4 说话，文字经转写、润色、格式化后自动落到光标位置。识别规则、专有词汇和输出风格都写在可编辑的 Markdown 提示词里，行业术语和个人说法一次调对。

它还支持**截图取字**：按 F6 框选屏幕任意区域，AI 理解版式后提取文字并复制到剪贴板。终端和 PDF 阅读器里的文字常常因为行号、分屏、硬换行被切碎，ByeType 能还原成可直接使用的完整段落。

免费开源，使用你自己的 API Key。语音和截图只发送到你选择的服务商，ByeType 本身不收费、不经手数据。支持 macOS、Windows 桌面端，以及 iPhone / iPad（通过 iOS 快捷指令）。

![录音 → 转写 → 优化 → 自动粘贴](docs/images/demo.gif)

## 🏆 为什么选择 ByeType

| | ByeType | 系统自带语音输入 | Whisper 类本地方案 |
|---|---|---|---|
| 安装体积 | **约 8 MB** | 系统内置 | 1~6 GB（模型文件） |
| 定制方式 | **编辑 Markdown 提示词** | 不支持 | 热词列表，仅提升识别率 |
| 人名与术语 | **提示词里写规则，转写时一次纠对** | 不支持 | 需后置 LLM 二次处理 |
| 格式化 | **数字、符号、大小写、自动换行** | 无 | 需后置处理 |
| 多场景切换 | **快捷键配模板，一键换风格** | 不支持 | 不支持 |
| 中英混合 | **优秀** | 差 | 一般 |

> **核心区别**：Whisper 类方案是「ASR 转文字 + LLM 后处理」两步架构，第一步转错的内容，第二步也救不回来。ByeType 用多模态大模型直接处理原始音频，所有提示词规则在转写时一次生效，没有「先错后纠」的问题。

## 🤖 支持的模型

预置 5 个模型：Qwen 3.5 Omni Plus / Flash 和 MiMo v2.5 走国内直连，Gemini 3.8 Flash 需要代理，DeepSeek V4.1 Flash 只做文本优化和截图取字。所有模型在「设置 → 模型管理」里填自己的 API Key。无法直接访问 Google 时，可以换成 OpenRouter 中转的 `google/gemini-3.8-flash` 或 `google/gemini-3.5-flash-lite`。

## 🔬 真实效果对比

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

## 📸 截图取字

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

## ✏️ 用 Markdown 自定义你的规则

ByeType 把所有「AI 该怎么处理你的话」都做成了可编辑的 Markdown 文件，在设置里直接改：转写提示词里写专有词汇和人名，文本优化提示词决定输出风格，图像识别提示词决定截图取字的处理方式。两个语音快捷键可以各绑一套风格，日常口语和正式书写一键切换。

```markdown
- 公司名：ByteDance（不是 byte dance）
- 人名：张三丰（不是 张三峰）
```

## 🧠 自动学习

说了十遍「张昱」，它还是转成「张宇」？改一次就够了。

ByeType 会对比你的原始转写和你修改后粘贴的文本，找出差异并归纳成纠错规则（如「张宇 → 张昱」），写进学习文档。之后的每次转写自动应用这些规则，越用越准。

- 从托盘菜单点「自动学习」，AI 对比后生成学习草稿，确认后写进学习文档
- 在「设置 → 自动学习」里查看和编辑学习结果，规则可随时增删
- 学习文档和转写提示词一样是 Markdown，支持手动修改

**规则增强**（设置 → 转写设置 → 其他）：如果模型转写纠错能力较弱，可以开启「规则增强」，让文本优化阶段再次注入所有转写规则做二次纠错。默认关闭，强模型转写时已一次纠对，无需重复。

## 🧰 进阶功能

### 本机 HTTP 接口

在「设置 → 通用设置 → 网络与性能」中首次开启本机转写接口后，其他本机程序可以复用 ByeType 当前的模型、代理、专有词汇、转写规则和语音模板。接口只监听 `127.0.0.1`，不会开放给局域网设备。

处理音频文件：

```bash
curl -fsS -X POST \
  --data-binary @recording.m4a \
  -H 'Content-Type: audio/mp4' \
  'http://127.0.0.1:8765/transcribe'
```

从标准输入读取音频，并指定已有模板：

```bash
some-audio-command | curl -fsS -X POST \
  --data-binary @- \
  -H 'Content-Type: audio/mpeg' \
  'http://127.0.0.1:8765/transcribe?template=voice-translate'
```

- 支持 M4A、MP3、WAV 和 FLAC
- 不传 `template` 时，使用第一语音快捷键绑定的模板
- 使用 `template=raw` 时，只执行转写，不执行第二阶段文本优化
- 成功响应只包含最终文本；错误通过 HTTP 状态码返回
- 接口调用不会显示气泡、修改剪贴板、自动粘贴或写入历史记录

### 备份与恢复

「设置 → 备份与恢复」支持把全部配置（API Key、提示词、学习结果、快捷键）备份到 S3 兼容存储，换电脑或重装系统时一键恢复。

### 📱 iPhone / iPad

通过 iOS 快捷指令，在手机和平板上也能获得和桌面版一样的自定义词汇和转录效果。

| 快捷指令 | 模型 | 安装 |
|---------|------|------|
| ByeType Gemini | Gemini 3.1 Flash Lite | [添加到快捷指令](https://www.icloud.com/shortcuts/85403a4e55a0487c985109f944f46ae8) |

> 安装后需要在快捷指令中填写你自己的 API Key 和规则词汇等，和桌面版共用同一个 Key。

## ❓ 常见问题

<details>
<summary><b>macOS 权限相关：没有声音、按 F4 没反应、文字没自动粘贴</b></summary>

这三类问题都是权限没开或没生效，在「系统设置 → 隐私与安全性」里检查：

- 没有声音 / 录音失败：麦克风权限
- 按 F4 没有反应：辅助功能权限
- 文本没有自动粘贴到输入框：辅助功能权限。文本仍会复制到剪贴板，可以手动 Cmd+V 粘贴

改完权限重启一次 ByeType。
</details>

<details>
<summary><b>macOS 提示「无法验证开发者」</b></summary>

前往「系统设置 → 隐私与安全性」，找到 ByeType 的提示信息，点击「仍要打开」。
</details>

<details>
<summary><b>按 F6 后没有识别结果</b></summary>

- 检查「设置 → 图像识别设置」里选的模型是否支持截图取字，见上文「支持的模型」
- 框选时按 Esc 会取消这次截图，不会产生结果
- 确认 API Key 有效、网络可达
</details>

<details>
<summary><b>转写结果为空</b></summary>

- 检查 API Key 是否正确填写
- 检查网络连接是否正常
- 如果使用 Gemini 模型，确认能访问 Google 服务（或已配置代理）
</details>

<details>
<summary><b>转写速度慢</b></summary>

- 关闭思考（设置 → 转写设置 → 启用思考 → 关闭）
- 切换更轻量的模型，比如 OpenRouter 的 `google/gemini-3.5-flash-lite`
- 检查网络延迟
</details>

<details>
<summary><b>国内网络无法使用 Gemini 模型</b></summary>

两种方案：

1. 在「设置 → 语音转写」中选择 Qwen 3.5 Omni 等国内直连模型，无需代理
2. 在「设置 → 通用设置 → 网络与性能 → HTTP 代理地址」中配置代理后使用 Gemini
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
