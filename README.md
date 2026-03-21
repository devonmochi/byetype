# ByeType

AI 驱动的语音输入工具，告别打字，用说的。

ByeType 是一个轻量级桌面应用，通过全局快捷键触发录音，利用 AI 将语音实时转录为文本，并支持 Markdown 格式优化。适合需要大量文字输入的写作、笔记、编程等场景。

## 功能特性

- **全局快捷键录音** — 随时随地一键开始/停止录音，不打断工作流
- **多模型语音转录** — 支持 Google Gemini、阿里云 Qwen 等多种 AI 模型
- **AI 文本优化** — 可选的二次处理，优化转录文本的格式和表达（支持 OpenAI 兼容 API）
- **思考模式** — 可配置的 AI 推理深度，在速度和质量间灵活选择
- **自定义提示词** — 内置提示词模板，支持自定义角色定义、转录规则、专有词汇
- **历史记录** — 保存所有录音和转录记录，支持查看、复制、重试
- **自动更新** — 内置应用更新检测，保持最新版本
- **系统托盘** — 常驻菜单栏，轻量不占空间

## 截图

> 截图即将添加

## 安装

从 [GitHub Releases](https://github.com/devonmochi/byetype/releases) 下载最新版本：

- **macOS**: 下载 `.dmg` 文件（支持 Apple Silicon 和 Intel）
- **Windows**: 下载 `.msi` 或 `.exe` 安装包

> macOS 用户首次打开可能需要在「系统设置 → 隐私与安全性」中允许运行。

## 从源码构建

### 环境要求

- [Node.js](https://nodejs.org/) >= 20
- [Rust](https://www.rust-lang.org/tools/install) >= 1.70
- [Tauri CLI](https://v2.tauri.app/start/prerequisites/) v2

### 步骤

```bash
# 克隆仓库
git clone https://github.com/devonmochi/byetype.git
cd byetype

# 安装前端依赖
npm install

# 启动开发模式
npm run tauri dev

# 构建生产版本
npm run tauri build
```

## 技术栈

- **框架**: [Tauri](https://v2.tauri.app/) v2
- **前端**: [React](https://react.dev/) 19 + TypeScript + [Vite](https://vite.dev/)
- **后端**: Rust
- **编辑器**: [CodeMirror](https://codemirror.net/) 6
- **AI**: Google Gemini API, 阿里云 Qwen API, OpenAI 兼容 API

## 许可证

[MIT](LICENSE)
