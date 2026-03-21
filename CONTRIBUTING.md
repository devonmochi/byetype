# 贡献指南

感谢你对 ByeType 的关注！欢迎通过以下方式参与贡献。

## 报告 Bug

请在 [Issues](https://github.com/devonmochi/byetype/issues) 中创建 Bug 报告，并包含：

- 操作系统及版本
- ByeType 版本
- 问题的详细描述和复现步骤
- 相关的错误日志或截图

## 功能建议

欢迎在 [Issues](https://github.com/devonmochi/byetype/issues) 中提交功能建议。请描述你期望的功能以及使用场景。

## 开发环境搭建

```bash
# 克隆仓库
git clone https://github.com/devonmochi/byetype.git
cd byetype

# 安装依赖
npm install

# 启动开发模式
npm run tauri dev
```

环境要求：
- Node.js >= 20
- Rust >= 1.70
- Tauri CLI v2

## 提交 Pull Request

1. Fork 本仓库
2. 创建功能分支（`git checkout -b feature/your-feature`）
3. 提交更改（`git commit -m 'feat: add your feature'`）
4. 推送到远程（`git push origin feature/your-feature`）
5. 创建 Pull Request

### Commit 规范

使用 [Conventional Commits](https://www.conventionalcommits.org/) 格式：

- `feat:` 新功能
- `fix:` Bug 修复
- `docs:` 文档更新
- `refactor:` 代码重构
- `chore:` 构建/工具变更

## 代码风格

- **TypeScript/React**: 遵循项目现有的代码风格
- **Rust**: 使用 `cargo fmt` 格式化，`cargo clippy` 检查
