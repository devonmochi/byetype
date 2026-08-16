# CLAUDE.md

## 📦 发版规范

- GitHub Release 的版本说明（release notes）必须使用中文撰写

## 🛠️ 开发命令

- 本地开发运行：`npm run tauri dev`

## 📝 修改提示词的落点

提示词有两份，改功能时**两份都要改**，只改一份会出现「代码里有、实际用不上」或「本机能用、别人装了没有」。

- **内置模板**：`src-tauri/prompts/*.md`，打包进 app 资源，影响新装用户和「恢复默认」，改动要提交进版本库
- **本机在用**：`~/Library/Application Support/com.byetype.app/prompts/*.md`，config.json 里 `voiceTemplates.templates[].prompt` 和 `transcribe.prompts` 指向它，app 实际读的是这份，不进版本库

两份内容可能已经被用户手工改过而不一致，改之前先分别读一遍，按各自的原文风格改，不要用模板整份覆盖本机那份。

改完提交时只提交 `src-tauri/prompts/` 下的改动。
