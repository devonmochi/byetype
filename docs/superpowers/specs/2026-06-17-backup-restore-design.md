# ByeType 备份与恢复功能设计

## 概述

为 ByeType 增加手动备份与恢复能力，支持 S3 兼容存储和本地文件两种方式，备份范围仅包含 `config.json` 和 `prompts/*.md`。

## 已确认需求

| 项目 | 决定 |
|---|---|
| 备份范围 | 仅 `config.json` + `prompts/*.md` |
| 备份目标 | S3 兼容存储 + 本地文件（每次弹对话框选位置） |
| S3 兼容性 | 支持任意 S3 兼容服务（AWS / R2 / B2 / MinIO / OSS 等） |
| 触发方式 | 纯手动（设置页点按钮） |
| 恢复功能 | S3 和本地都支持恢复 |
| 备份格式 | zip（如 `byetype-backup-20260617-153000.zip`） |
| S3 路径 | 固定前缀 + 时间戳 `byetype/backups/byetype-backup-XXXXXX.zip` |

## 技术方案

选定方案 A：使用 Rust `s3` crate 轻量 S3 客户端 + `zip` crate 打包/解压。

理由：与现有 `reqwest` + `tokio` 集成好，二进制体积增加小，维护成本低，备份场景完全够用。

## 整体架构

### 新增文件

```
src-tauri/src/
  backup/
    mod.rs          — 模块入口
    archive.rs      — zip 打包/解压逻辑
    s3.rs           — S3 上传/下载/列表
    local.rs        — 本地导出/导入（文件对话框）

src/views/settings/tabs/
    Backup.tsx      — 备份与恢复设置页
```

### 备份数据流

收集 `config.json` + `prompts/*.md` → 打包 zip → 上传 S3 或保存到用户选择的本地路径

### 恢复数据流

从 S3 下载 zip 或用户选择本地 zip → 解压到临时目录 → 校验 → 覆盖 app_data_dir 下对应文件 → 提示重启生效

### 关键决策

- S3 操作全部在 Rust 后端完成（避免前端处理二进制数据和签名）
- 前端通过 Tauri command 调用后端
- 恢复 config.json 后需要重启应用才能生效（会提示用户）

## S3 配置与凭证存储

在 `config.json` 中新增 `backup` 字段：

```json
{
  "backup": {
    "s3": {
      "endpoint": "https://s3.amazonaws.com",
      "region": "us-east-1",
      "bucket": "my-backup-bucket",
      "access_key": "AKIAXXXXX",
      "secret_key": "xxxxxxx",
      "prefix": "byetype/backups"
    }
  }
}
```

| 字段 | 说明 | 默认值 |
|---|---|---|
| `endpoint` | S3 兼容服务地址，空则用 AWS 标准 | 空 |
| `region` | 区域 | — |
| `bucket` | 存储桶名 | — |
| `access_key` | Access Key ID | — |
| `secret_key` | Secret Access Key | — |
| `prefix` | S3 对象路径前缀 | `byetype/backups` |

凭证与现有 API key 一样明文存储在 `config.json` 中（保持与项目现有方式一致，不额外引入加密）。

前端设置页提供测试连接按钮，后端执行 `HeadBucket` 请求验证凭证和 bucket 可达性。

## Tauri Commands 接口

后端暴露以下 IPC 命令：

| 命令 | 方向 | 说明 |
|---|---|---|
| `backup_to_s3` | S3 备份 | 打包 zip → 上传 S3 → 返回对象 key |
| `restore_from_s3` | S3 恢复 | 下载 zip → 解压 → 覆盖本地 |
| `list_s3_backups` | S3 列表 | 返回备份列表 [{key, size, last_modified}] |
| `test_s3_connection` | S3 测试 | HeadBucket 验证连接 |
| `backup_to_local` | 本地备份 | 打包 zip → 弹文件对话框 → 保存 |
| `restore_from_local` | 本地恢复 | 弹对话框选 zip → 解压 → 覆盖 |

### BackupEntry 结构

```json
{
  "key": "byetype/backups/byetype-backup-20260617-153000.zip",
  "size": 15234,
  "last_modified": "2026-06-17T15:30:00Z"
}
```

### 要点

- 本地备份/恢复使用 `tauri-plugin-dialog` 文件对话框（前端已有依赖 `@tauri-apps/plugin-dialog`）
- 所有命令在后台异步执行，前端显示 loading 状态
- 恢复操作完成后弹窗提示「需要重启应用才能生效」

## Rust 依赖新增

```toml
# Cargo.toml 新增
s3 = "0.13"       # 或最新稳定版，支持自定义 endpoint
zip = "2.0"       # zip 打包/解压
```

## 前端设置页 UI

新增「备份与恢复」tab，包含：

1. **S3 配置区**：endpoint、region、bucket、access_key、secret_key、prefix 输入框 + 测试连接按钮
2. **S3 操作区**：立即备份按钮、备份列表（可选择恢复）、刷新列表按钮
3. **本地操作区**：备份到本地按钮、从本地恢复按钮

## 错误处理

- S3 连接失败：返回具体错误信息（网络/认证/bucket 不存在）
- zip 打包失败：返回 IO 错误信息
- 恢复时校验失败：zip 内缺少 `config.json` 则拒绝恢复，提示文件不合法
- 恢复前自动备份当前配置到临时文件，恢复失败时回滚
