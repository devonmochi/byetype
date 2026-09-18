# ByeType

[![License: MIT](https://img.shields.io/badge/license-MIT-blue?style=flat-square)](LICENSE)
[![Platform](https://img.shields.io/badge/platform-macOS%20%7C%20Windows%20%7C%20iOS-brightgreen?style=flat-square)](https://github.com/devonmochi/byetype/releases)

ByeType是一个由Markdown驱动的AI语音输入工具。你可以自由定义规则、设置专有词汇和输出风格，不受传统输入法固定词库和有限自定义项的约束。模型直接理解原始音频，并在转写时处理人名、行业术语、数字格式、口语清理和中英混合内容。你可以闭着眼睛说完直接发送，不用检查识别结果是否正确，做到说什么就发什么。

ByeType免费开源，使用你自己的API Key。支持macOS、Windows，以及通过iOS快捷指令在iPhone和iPad上使用。

![录音 → 转写 → 优化 → 自动粘贴](docs/images/demo.gif)

## 📱 iPhone / iPad

通过iOS快捷指令，在手机和平板上也能获得和桌面版一样的自定义词汇和转录效果。

| 快捷指令 | 模型 | 安装 |
|---------|------|------|
| Byetype Qwen | Qwen 3.8 Omni Flash | [添加到快捷指令](https://www.icloud.com/shortcuts/7db041829de14faa964b02369a23d8cc) |

> 安装后需要在快捷指令中填写你自己的API Key和规则词汇等，和桌面版共用同一个Key。

## 🏆 为什么选择 ByeType

| | ByeType | 系统自带语音输入 | Whisper 类本地方案 |
|---|---|---|---|
| 安装体积 | **约8MB** | 系统内置 | 1~6GB（模型文件） |
| 定制方式 | **编辑Markdown提示词** | 不支持 | 只能添加热词 |
| 人名与术语 | **转写时按提示词校正** | 不支持 | 需要LLM再次处理 |
| 格式化 | **数字、符号、大小写、自动换行** | 无 | 需要LLM再次处理 |
| 多场景切换 | **每个快捷键可绑定一种输出风格** | 不支持 | 不支持 |
| 中英混合 | **可按词汇表校正** | 无法自定义 | 只能添加热词 |

> Whisper类方案先把语音转成文字，再让LLM处理文字。ByeType让多模态大模型直接处理原始音频，转写时同时应用提示词规则。

## 🔬 真实效果对比

### 普通语音输出

有标点但不分段、人名全错、术语变谐音、口水词原样保留：

> 嗯，那个就是昨天<u>**张宇**</u>跟<u>**秦敏**</u>碰了一下，他们说<u>**曲华**</u>负责的那个<u>**deep seek v3**</u>项目，在<u>**mac mini m4**</u>上跑<u>**因弗伦斯**</u>延迟大概<u>**两百毫秒**</u>左右，效果还不错。嗯，然后<u>**余倩**</u>建议用<u>**cursor**</u>开发，后端<u>**fast api**</u>加<u>**泼斯特格瑞赛口**</u>部署在<u>**微赛尔**</u>上，前端<u>**next js**</u>用<u>**app router**</u>搭配<u>**莎德恩ui**</u>，整体<u>**dx**</u>我觉得还行，就是<u>**ci cd**</u>那块<u>**git hub actions**</u>跑<u>**派test**</u>和<u>**es lint**</u>经常<u>**飞来可test**</u>。然后<u>**库伯内提斯**</u>集群的<u>**hpa**</u>配置，<u>**余倩**</u>说应该把<u>**cpu**</u>阈值从<u>**百分之八十**</u>调到<u>**百分之六十**</u>。另外提醒一下<u>**陈述**</u>，礼拜五之前把<u>**非格码**</u>设计稿同步到<u>**诺讯**</u>上。

### ByeType 输出

开启「自动换行」后，ByeType会处理标点、分段、人名、术语和格式：

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
| 自动分段 | 没有分段 | 5个自然段落 |

## 📸 截图取字

按**F6**框选截图后，ByeType识别文字并复制到剪贴板。你可以编辑Markdown提示词（`text-extract.md`），定义它如何处理截图。ByeType会理解截图布局并修复断行、行号和界面装饰。

### 示例1：修复换行

终端、浏览器和PDF阅读器会按窗口宽度截断文字。传统OCR保留这些断行，ByeType会按语义合并成完整段落。

#### 传统 OCR 输出

逐行照搬，保留所有因窗口宽度产生的硬换行：

> 人工智能(AI)正在迅速发展，它已经开始<br>改变我们的生活方式和工作方式。从智能<br>手机助手到自动驾驶汽车，AI技术正在<br>各个领域展现其潜力。

#### ByeType 输出

理解语义，自动合并断行为完整段落：

> 人工智能(AI)正在迅速发展，它已经开始改变我们的生活方式和工作方式。从智能手机助手到自动驾驶汽车，AI技术正在各个领域展现其潜力。

### 示例2：还原终端代码

在Claude Code、终端和IDE里截图代码时，行号、提示符和分屏边界会混入代码。ByeType会去掉这些内容，恢复可直接使用的代码块。

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

ByeType把转写要求保存在可编辑的Markdown文件中。你可以在转写提示词中填写专有词汇和人名，用文本优化提示词定义输出风格，用图像识别提示词定义截图取字的处理方式。两个语音快捷键可以分别绑定一种输出风格。在一句话末尾说出`Chinese to English`，ByeType会把前面的内容翻译成英文。

```markdown
- 公司名：ByteDance（不是byte dance）
- 人名：张三丰（不是张三峰）
```

## 🧠 自动学习

如果ByeType把「张昱」写成「张宇」，你修改一次即可。ByeType会对比原始转写和修改后的文本，把「张宇→张昱」等差异写成纠错规则，之后转写时自动应用。

- 从托盘菜单点「自动学习」，AI 对比后生成学习草稿，确认后写进学习文档
- 在「设置 → 自动学习」里查看和编辑学习结果，规则可随时增删
- 学习文档和转写提示词一样是 Markdown，支持手动修改

**规则增强**（设置→转写设置→其他）：如果转写模型没有按规则纠错，可以开启「规则增强」，让文本优化模型再按转写规则处理一次。此选项默认关闭。

## 🧰 进阶功能

### 本机 HTTP 接口

在「设置→通用设置→网络与性能」中开启本机转写接口后，其他本机程序可以使用ByeType当前的模型、代理、专有词汇、转写规则和语音模板。接口只监听`127.0.0.1`，局域网内的其他设备无法访问。

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

「设置→备份与恢复」可以把API Key、提示词、学习结果和快捷键备份到S3兼容存储，换电脑或重装系统后可以恢复。

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

1. 在「设置 → 语音转写」中选择Qwen 3.8 Omni Flash等国内直连模型，无需代理
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

欢迎提交Issue或PR。感谢[Linux.do](https://linux.do/)社区推动。
