# ByeType 备份与恢复功能实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 为 ByeType 添加 S3 兼容存储 + 本地 zip 备份/恢复功能，覆盖 config.json 和 prompts/*.md。

**Architecture:** Rust 后端新增 `backup` 模块（archive + s3 + local），通过 Tauri commands 暴露给前端；前端在设置页新增「备份与恢复」tab，包含 S3 配置、测试连接、S3 备份/恢复、本地备份/恢复。

**Tech Stack:** Rust (s3 crate, zip crate, tokio, reqwest), React + TypeScript (Tauri IPC, @tauri-apps/plugin-dialog)

## Global Constraints

- 所有 Rust 结构体使用 `#[derive(Debug, Clone, Serialize, Deserialize)]` + `#[serde(rename_all = "camelCase")]`
- 前端 TypeScript 类型与 Rust 结构体保持 camelCase 一一对应
- Tauri 命令注册在 `src-tauri/src/lib.rs` 的 `generate_handler!` 宏中
- 前端 invoke 调用封装在 `src/lib/tauri-api.ts` 中
- 设置页 tab 注册在 `src/views/settings/App.tsx` 的 `TABS` 数组和条件渲染中
- 备份文件名格式：`byetype-backup-YYYYMMDD-HHMMSS.zip`
- S3 对象路径：`{prefix}/byetype-backup-YYYYMMDD-HHMMSS.zip`，默认 prefix 为 `byetype/backups`
- commit message 使用中文
- 测试命令：`cd src-tauri && cargo test`

---

## File Structure

```
src-tauri/Cargo.toml                          — 新增 s3, zip 依赖
src-tauri/src/config/types.rs                 — 新增 BackupConfig, S3Config 结构体，AppConfig 新增 backup 字段
src-tauri/src/backup/mod.rs                   — 模块入口
src-tauri/src/backup/archive.rs               — zip 打包/解压逻辑
src-tauri/src/backup/s3.rs                    — S3 上传/下载/列表/测试连接
src-tauri/src/backup/local.rs                 — 本地导出/导入
src-tauri/src/commands.rs                     — 新增 6 个 backup 相关 Tauri commands
src-tauri/src/lib.rs                          — 注册 backup 模块和命令
src/core/types.ts                             — 新增 BackupConfig, S3Config, BackupEntry 类型
src/lib/tauri-api.ts                          — 新增 backup API 函数
src/views/settings/tabs/BackupTab.tsx         — 备份与恢复设置页 UI
src/views/settings/App.tsx                    — 注册 backup tab
```

---

### Task 1: 新增 Rust 依赖和配置类型

**Files:**
- Modify: `src-tauri/Cargo.toml`
- Modify: `src-tauri/src/config/types.rs`

**Interfaces:**
- Produces: `BackupConfig` struct with `s3: Option<S3Config>`, `S3Config` struct with `endpoint`, `region`, `bucket`, `access_key`, `secret_key`, `prefix` fields

- [ ] **Step 1: 在 Cargo.toml 添加 s3 和 zip 依赖**

在 `src-tauri/Cargo.toml` 的 `[dependencies]` 末尾（`tokio-util = "0.7"` 之后）添加：

```toml
s3 = { version = "0.13", default-features = false, features = ["tokio-rustls"] }
zip = "2.0"
```

注意：使用 `tokio-rustls` feature 避免引入 OpenSSL 依赖，保持跨平台编译一致性。`default-features = false` 关闭同步和 native-tls。

- [ ] **Step 2: 在 config/types.rs 添加 S3Config 和 BackupConfig 结构体**

在 `src-tauri/src/config/types.rs` 的 `AdvancedConfig` 结构体之后（第 218 行后）、`impl Default for AppConfig` 之前添加：

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct S3Config {
    #[serde(default)]
    pub endpoint: String,
    #[serde(default)]
    pub region: String,
    #[serde(default)]
    pub bucket: String,
    #[serde(default)]
    pub access_key: String,
    #[serde(default)]
    pub secret_key: String,
    #[serde(default = "default_s3_prefix")]
    pub prefix: String,
}

fn default_s3_prefix() -> String {
    "byetype/backups".to_string()
}

impl Default for S3Config {
    fn default() -> Self {
        Self {
            endpoint: String::new(),
            region: String::new(),
            bucket: String::new(),
            access_key: String::new(),
            secret_key: String::new(),
            prefix: default_s3_prefix(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct BackupConfig {
    #[serde(default)]
    pub s3: S3Config,
}
```

- [ ] **Step 3: 在 AppConfig 添加 backup 字段**

在 `src-tauri/src/config/types.rs` 的 `AppConfig` 结构体中，`advanced` 字段之后添加 `backup` 字段：

修改第 5-14 行的 `AppConfig`：

```rust
pub struct AppConfig {
    pub general: GeneralConfig,
    pub models: ModelsConfig,
    pub transcribe: TranscribeConfig,
    #[serde(alias = "optimize")]
    pub voice_templates: VoiceTemplatesConfig,
    #[serde(default)]
    pub extract: ExtractConfig,
    pub advanced: AdvancedConfig,
    #[serde(default)]
    pub backup: BackupConfig,
}
```

- [ ] **Step 4: 在 Default impl 中添加 backup 默认值**

在 `impl Default for AppConfig` 的 `advanced` 字段后（第 285 行 `proxy_url: String::new(),` 之后），闭合括号之前添加：

```rust
            backup: BackupConfig::default(),
```

- [ ] **Step 5: 验证编译通过**

Run: `cd /Users/lishaojie/PycharmProjects/pythonProject1/日常工具/byetype/src-tauri && cargo check`
Expected: 编译成功，无错误

- [ ] **Step 6: Commit**

```bash
cd /Users/lishaojie/PycharmProjects/pythonProject1/日常工具/byetype
git add src-tauri/Cargo.toml src-tauri/Cargo.lock src-tauri/src/config/types.rs
git commit -m "feat(backup): 添加 S3 和 zip 依赖及 BackupConfig 配置类型"
```

---

### Task 2: 实现 zip 打包/解压模块

**Files:**
- Create: `src-tauri/src/backup/mod.rs`
- Create: `src-tauri/src/backup/archive.rs`

**Interfaces:**
- Produces: `pub fn create_backup_zip(data_dir: &Path) -> Result<Vec<u8>, String>` — 将 config.json + prompts/*.md 打包为 zip 字节
- Produces: `pub fn extract_backup_zip(zip_data: &[u8], dest_dir: &Path) -> Result<(), String>` — 解压 zip 到目标目录

- [ ] **Step 1: 创建 backup/mod.rs**

创建 `src-tauri/src/backup/mod.rs`：

```rust
pub mod archive;
pub mod s3;
pub mod local;
```

- [ ] **Step 2: 创建 backup/archive.rs 实现 create_backup_zip**

创建 `src-tauri/src/backup/archive.rs`：

```rust
use std::fs;
use std::io::{Read, Write};
use std::path::Path;
use zip::ZipWriter;
use zip::write::SimpleFileOptions;
use zip::CompressionMethod;

/// 将 config.json 和 prompts/ 目录下的 .md 文件打包为 zip 字节
pub fn create_backup_zip(data_dir: &Path) -> Result<Vec<u8>, String> {
    let mut buffer = Vec::new();
    {
        let mut zip = ZipWriter::new(std::io::Cursor::new(&mut buffer));
        let options = SimpleFileOptions::default()
            .compression_method(CompressionMethod::Deflated);

        // 添加 config.json
        let config_path = data_dir.join("config.json");
        if config_path.exists() {
            let config_content = fs::read(&config_path)
                .map_err(|e| format!("读取 config.json 失败: {}", e))?;
            zip.start_file("config.json", options)
                .map_err(|e| format!("写入 zip 失败: {}", e))?;
            zip.write_all(&config_content)
                .map_err(|e| format!("写入 zip 失败: {}", e))?;
        }

        // 添加 prompts/ 目录下的所有文件
        let prompts_dir = data_dir.join("prompts");
        if prompts_dir.exists() {
            let entries = collect_files(&prompts_dir)
                .map_err(|e| format!("遍历 prompts 目录失败: {}", e))?;
            for (relative_path, full_path) in entries {
                let content = fs::read(&full_path)
                    .map_err(|e| format!("读取文件 {} 失败: {}", relative_path, e))?;
                zip.start_file(format!("prompts/{}", relative_path), options)
                    .map_err(|e| format!("写入 zip 失败: {}", e))?;
                zip.write_all(&content)
                    .map_err(|e| format!("写入 zip 失败: {}", e))?;
            }
        }

        zip.finish()
            .map_err(|e| format!("完成 zip 失败: {}", e))?;
    }

    Ok(buffer)
}

/// 递归收集目录下所有文件，返回 (相对路径, 完整路径) 列表
fn collect_files(dir: &Path) -> std::io::Result<Vec<(String, std::path::PathBuf)>> {
    let mut result = Vec::new();
    let entries = fs::read_dir(dir)?;
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            let sub = collect_files(&path)?;
            for (rel, full) in sub {
                let parent_name = entry.file_name().to_string_lossy().to_string();
                result.push((format!("{}/{}", parent_name, rel), full));
            }
        } else {
            let name = entry.file_name().to_string_lossy().to_string();
            result.push((name, path));
        }
    }
    Ok(result)
}

/// 解压 zip 数据到目标目录
pub fn extract_backup_zip(zip_data: &[u8], dest_dir: &Path) -> Result<(), String> {
    let reader = std::io::Cursor::new(zip_data);
    let mut archive = zip::ZipArchive::new(reader)
        .map_err(|e| format!("打开 zip 失败: {}", e))?;

    // 校验：zip 中必须包含 config.json
    let has_config = (0..archive.len()).any(|i| {
        archive.by_index(i)
            .map(|f| f.name().to_string() == "config.json")
            .unwrap_or(false)
    });
    if !has_config {
        return Err("备份文件无效：缺少 config.json".to_string());
    }

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)
            .map_err(|e| format!("读取 zip 条目失败: {}", e))?;

        let path = match file.enclosed_name() {
            Some(p) => dest_dir.join(p),
            None => continue,
        };

        if file.is_dir() {
            fs::create_dir_all(&path)
                .map_err(|e| format!("创建目录失败: {}", e))?;
        } else {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)
                    .map_err(|e| format!("创建目录失败: {}", e))?;
            }
            let mut out = fs::File::create(&path)
                .map_err(|e| format!("创建文件失败: {}", e))?;
            let mut buf = Vec::new();
            file.read_to_end(&mut buf)
                .map_err(|e| format!("读取 zip 条目失败: {}", e))?;
            out.write_all(&buf)
                .map_err(|e| format!("写入文件失败: {}", e))?;
        }
    }

    Ok(())
}
```

- [ ] **Step 3: 在 lib.rs 注册 backup 模块**

在 `src-tauri/src/lib.rs` 的 mod 声明区域（第 11 行 `mod updater;` 之后）添加：

```rust
mod backup;
```

- [ ] **Step 4: 创建 backup/s3.rs 和 backup/local.rs 占位**

创建 `src-tauri/src/backup/s3.rs`：

```rust
// S3 操作将在 Task 3 实现
```

创建 `src-tauri/src/backup/local.rs`：

```rust
// 本地操作将在 Task 4 实现
```

- [ ] **Step 5: 验证编译通过**

Run: `cd /Users/lishaojie/PycharmProjects/pythonProject1/日常工具/byetype/src-tauri && cargo check`
Expected: 编译成功

- [ ] **Step 6: Commit**

```bash
cd /Users/lishaojie/PycharmProjects/pythonProject1/日常工具/byetype
git add src-tauri/src/backup/ src-tauri/src/lib.rs
git commit -m "feat(backup): 实现 zip 打包和解压模块"
```

---

### Task 3: 实现 S3 操作模块

**Files:**
- Modify: `src-tauri/src/backup/s3.rs`

**Interfaces:**
- Consumes: `S3Config` from `config/types.rs`
- Produces: `pub async fn test_connection(config: &S3Config) -> Result<(), String>`
- Produces: `pub async fn upload(config: &S3Config, data: Vec<u8>, key: String) -> Result<String, String>`
- Produces: `pub async fn download(config: &S3Config, key: &str) -> Result<Vec<u8>, String>`
- Produces: `pub async fn list_backups(config: &S3Config) -> Result<Vec<BackupEntry>, String>`
- Produces: `pub struct BackupEntry { key: String, size: u64, last_modified: String }`

- [ ] **Step 1: 实现 backup/s3.rs 完整内容**

将 `src-tauri/src/backup/s3.rs` 替换为：

```rust
use serde::{Deserialize, Serialize};
use s3::{Bucket, Region};
use s3::creds::Credentials;
use crate::config::types::S3Config;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupEntry {
    pub key: String,
    pub size: u64,
    pub last_modified: String,
}

fn create_bucket(config: &S3Config) -> Result<Bucket, String> {
    let region = if config.endpoint.is_empty() {
        config.region
            .parse::<Region>()
            .map_err(|e| format!("区域配置无效: {}", e))?
    } else {
        Region::Custom {
            region: config.region.clone(),
            endpoint: config.endpoint.clone(),
        }
    };

    let credentials = Credentials::new(
        Some(&config.access_key),
        Some(&config.secret_key),
        None,
        None,
        None,
    )
    .map_err(|e| format!("凭证创建失败: {}", e))?;

    let bucket = Bucket::new(&config.bucket, region, credentials)
        .map_err(|e| format!("创建 bucket 失败: {}", e))?;

    // 对自定义 endpoint 使用 path style（兼容 MinIO / R2 等）
    if !config.endpoint.is_empty() {
        Ok(bucket.with_path_style())
    } else {
        Ok(bucket)
    }
}

pub async fn test_connection(config: &S3Config) -> Result<(), String> {
    if config.bucket.is_empty() {
        return Err("未配置 bucket".to_string());
    }
    if config.access_key.is_empty() || config.secret_key.is_empty() {
        return Err("未配置 Access Key 或 Secret Key".to_string());
    }

    let bucket = create_bucket(config)?;
    let (_, status) = bucket.head_bucket()
        .await
        .map_err(|e| format!("连接 S3 失败: {}", e))?;

    if status == 200 {
        Ok(())
    } else {
        Err(format!("S3 连接测试失败，状态码: {}", status))
    }
}

pub async fn upload(config: &S3Config, data: Vec<u8>, key: String) -> Result<String, String> {
    let bucket = create_bucket(config)?;
    let response = bucket.put_object(&key, &data)
        .await
        .map_err(|e| format!("上传 S3 失败: {}", e))?;

    if response.status_code() == 200 {
        Ok(key)
    } else {
        Err(format!("上传失败，状态码: {}", response.status_code()))
    }
}

pub async fn download(config: &S3Config, key: &str) -> Result<Vec<u8>, String> {
    let bucket = create_bucket(config)?;
    let response = bucket.get_object(key)
        .await
        .map_err(|e| format!("从 S3 下载失败: {}", e))?;

    if response.status_code() == 200 {
        Ok(response.to_vec())
    } else {
        Err(format!("下载失败，状态码: {}", response.status_code()))
    }
}

pub async fn list_backups(config: &S3Config) -> Result<Vec<BackupEntry>, String> {
    let bucket = create_bucket(config)?;
    let prefix = if config.prefix.is_empty() {
        "byetype/backups".to_string()
    } else {
        config.prefix.clone()
    };

    let results = bucket.list(prefix, None)
        .await
        .map_err(|e| format!("列出 S3 备份失败: {}", e))?;

    let mut entries = Vec::new();
    for list in results {
        for obj in list.contents {
            // 只返回 zip 文件
            if obj.key.ends_with(".zip") {
                entries.push(BackupEntry {
                    key: obj.key,
                    size: obj.size,
                    last_modified: obj.last_modified,
                });
            }
        }
    }

    // 按时间倒序排列（最新的在前）
    entries.sort_by(|a, b| b.last_modified.cmp(&a.last_modified));

    Ok(entries)
}

/// 生成带时间戳的 S3 对象 key
pub fn generate_backup_key(prefix: &str) -> String {
    let now = chrono::Local::now();
    let timestamp = now.format("%Y%m%d-%H%M%S").to_string();
    let prefix = if prefix.is_empty() { "byetype/backups" } else { prefix };
    format!("{}/byetype-backup-{}.zip", prefix, timestamp)
}
```

注意：这里用到了 `chrono` crate 的 `Local::now()`。需要在 Cargo.toml 添加 chrono 依赖。`s3` crate 已包含 `chrono` 作为传递依赖，但直接使用需要显式声明。

- [ ] **Step 2: 在 Cargo.toml 添加 chrono 依赖**

在 `src-tauri/Cargo.toml` 的 `[dependencies]` 中 `zip = "2.0"` 之后添加：

```toml
chrono = "0.4"
```

- [ ] **Step 3: 验证编译通过**

Run: `cd /Users/lishaojie/PycharmProjects/pythonProject1/日常工具/byetype/src-tauri && cargo check`
Expected: 编译成功

- [ ] **Step 4: Commit**

```bash
cd /Users/lishaojie/PycharmProjects/pythonProject1/日常工具/byetype
git add src-tauri/src/backup/s3.rs src-tauri/Cargo.toml src-tauri/Cargo.lock
git commit -m "feat(backup): 实现 S3 上传/下载/列表/测试连接"
```

---

### Task 4: 实现本地备份/恢复模块

**Files:**
- Modify: `src-tauri/src/backup/local.rs`

**Interfaces:**
- Consumes: `create_backup_zip` and `extract_backup_zip` from `backup/archive.rs`
- Produces: `pub fn backup_to_local(data_dir: &Path, save_path: &Path) -> Result<String, String>` — 打包并写入指定路径
- Produces: `pub fn restore_from_local(zip_path: &Path, data_dir: &Path) -> Result<(), String>` — 从 zip 恢复，先备份当前 config

- [ ] **Step 1: 实现 backup/local.rs 完整内容**

将 `src-tauri/src/backup/local.rs` 替换为：

```rust
use std::fs;
use std::path::Path;
use crate::backup::archive::{create_backup_zip, extract_backup_zip};

/// 备份到本地指定路径
pub fn backup_to_local(data_dir: &Path, save_path: &Path) -> Result<String, String> {
    let zip_data = create_backup_zip(data_dir)?;

    // 确保父目录存在
    if let Some(parent) = save_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("创建目录失败: {}", e))?;
    }

    fs::write(save_path, &zip_data)
        .map_err(|e| format!("写入备份文件失败: {}", e))?;

    Ok(save_path.to_string_lossy().to_string())
}

/// 从本地 zip 恢复
/// 先备份当前 config.json 到 config.json.bak，恢复失败时回滚
pub fn restore_from_local(zip_path: &Path, data_dir: &Path) -> Result<(), String> {
    let zip_data = fs::read(zip_path)
        .map_err(|e| format!("读取备份文件失败: {}", e))?;

    // 备份当前 config.json
    let config_path = data_dir.join("config.json");
    let backup_config_path = data_dir.join("config.json.bak");
    let has_existing_config = config_path.exists();

    if has_existing_config {
        fs::copy(&config_path, &backup_config_path)
            .map_err(|e| format!("备份当前配置失败: {}", e))?;
    }

    // 解压到临时目录
    let temp_dir = data_dir.join(".restore_temp");
    if temp_dir.exists() {
        fs::remove_dir_all(&temp_dir)
            .map_err(|e| format!("清理临时目录失败: {}", e))?;
    }

    match extract_backup_zip(&zip_data, &temp_dir) {
        Ok(()) => {
            // 用临时目录中的文件覆盖实际文件
            // config.json
            let temp_config = temp_dir.join("config.json");
            if temp_config.exists() {
                fs::copy(&temp_config, &config_path)
                    .map_err(|e| format!("覆盖 config.json 失败: {}", e))?;
            }

            // prompts/
            let temp_prompts = temp_dir.join("prompts");
            let actual_prompts = data_dir.join("prompts");
            if temp_prompts.exists() {
                if actual_prompts.exists() {
                    fs::remove_dir_all(&actual_prompts)
                        .map_err(|e| format!("清理旧 prompts 失败: {}", e))?;
                }
                copy_dir_recursive(&temp_prompts, &actual_prompts)
                    .map_err(|e| format!("恢复 prompts 失败: {}", e))?;
            }

            // 清理临时目录
            let _ = fs::remove_dir_all(&temp_dir);

            // 清理备份
            if has_existing_config {
                let _ = fs::remove_file(&backup_config_path);
            }

            Ok(())
        }
        Err(e) => {
            // 恢复失败，回滚
            let _ = fs::remove_dir_all(&temp_dir);
            if has_existing_config && backup_config_path.exists() {
                let _ = fs::copy(&backup_config_path, &config_path);
                let _ = fs::remove_file(&backup_config_path);
            }
            Err(format!("恢复失败（已回滚）: {}", e))
        }
    }
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> std::io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}
```

- [ ] **Step 2: 验证编译通过**

Run: `cd /Users/lishaojie/PycharmProjects/pythonProject1/日常工具/byetype/src-tauri && cargo check`
Expected: 编译成功

- [ ] **Step 3: Commit**

```bash
cd /Users/lishaojie/PycharmProjects/pythonProject1/日常工具/byetype
git add src-tauri/src/backup/local.rs
git commit -m "feat(backup): 实现本地备份和恢复逻辑"
```

---

### Task 5: 实现 Tauri Commands

**Files:**
- Modify: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/lib.rs`

**Interfaces:**
- Consumes: all `backup` module functions, `ConfigManager`, `AppConfig`
- Produces: 6 Tauri commands — `backup_to_s3`, `restore_from_s3`, `list_s3_backups`, `test_s3_connection`, `backup_to_local`, `restore_from_local`

- [ ] **Step 1: 在 commands.rs 添加 backup 命令**

在 `src-tauri/src/commands.rs` 文件末尾添加以下内容：

```rust
// ==================== Backup ====================

use crate::backup::s3 as s3_backup;
use crate::backup::local as local_backup;
use crate::backup::archive::{create_backup_zip, extract_backup_zip};
use crate::config::types::S3Config;
use tauri_plugin_dialog::DialogExt;

#[tauri::command]
pub async fn test_s3_connection(config_manager: State<'_, ConfigManager>) -> Result<(), String> {
    let config = config_manager.get();
    let s3_config = &config.backup.s3;
    s3_backup::test_connection(s3_config).await
}

#[tauri::command]
pub async fn backup_to_s3(
    app: tauri::AppHandle,
    config_manager: State<'_, ConfigManager>,
) -> Result<String, String> {
    let config = config_manager.get();
    let s3_config = &config.backup.s3;

    if s3_config.bucket.is_empty() {
        return Err("请先在设置中配置 S3".to_string());
    }

    let data_dir = app.path().app_data_dir()
        .map_err(|e| format!("获取数据目录失败: {}", e))?;

    let zip_data = create_backup_zip(&data_dir)?;
    let key = s3_backup::generate_backup_key(&s3_config.prefix);

    s3_backup::upload(s3_config, zip_data, key).await
}

#[tauri::command]
pub async fn list_s3_backups(
    config_manager: State<'_, ConfigManager>,
) -> Result<Vec<s3_backup::BackupEntry>, String> {
    let config = config_manager.get();
    let s3_config = &config.backup.s3;

    if s3_config.bucket.is_empty() {
        return Err("请先在设置中配置 S3".to_string());
    }

    s3_backup::list_backups(s3_config).await
}

#[tauri::command]
pub async fn restore_from_s3(
    app: tauri::AppHandle,
    config_manager: State<'_, ConfigManager>,
    object_key: String,
) -> Result<(), String> {
    let config = config_manager.get();
    let s3_config = &config.backup.s3;

    let zip_data = s3_backup::download(s3_config, &object_key).await?;

    let data_dir = app.path().app_data_dir()
        .map_err(|e| format!("获取数据目录失败: {}", e))?;

    // 使用 local_backup 的恢复逻辑（它处理回滚）
    // 先写入临时文件，再调用 restore_from_local
    let temp_zip = data_dir.join(".s3_restore_temp.zip");
    std::fs::write(&temp_zip, &zip_data)
        .map_err(|e| format!("写入临时文件失败: {}", e))?;

    let result = local_backup::restore_from_local(&temp_zip, &data_dir);

    // 清理临时文件
    let _ = std::fs::remove_file(&temp_zip);

    result
}

#[tauri::command]
pub async fn backup_to_local(
    app: tauri::AppHandle,
) -> Result<String, String> {
    let data_dir = app.path().app_data_dir()
        .map_err(|e| format!("获取数据目录失败: {}", e))?;

    let now = chrono::Local::now();
    let timestamp = now.format("%Y%m%d-%H%M%S").to_string();
    let default_name = format!("byetype-backup-{}.zip", timestamp);

    // 弹出文件保存对话框
    let (sender, receiver) = std::sync::mpsc::channel();
    app.dialog()
        .file()
        .add_filter("ZIP 文件", &["zip"])
        .set_file_name(&default_name)
        .save_file(move |file_path| {
            let _ = sender.send(file_path);
        });

    let file_path = receiver.recv()
        .map_err(|_| "文件对话框取消或超时".to_string())?;

    let path = match file_path {
        Some(p) => p,
        None => return Err("未选择保存位置".to_string()),
    };

    let save_path = std::path::PathBuf::from(path.to_string());
    local_backup::backup_to_local(&data_dir, &save_path)
}

#[tauri::command]
pub async fn restore_from_local(
    app: tauri::AppHandle,
) -> Result<(), String> {
    let data_dir = app.path().app_data_dir()
        .map_err(|e| format!("获取数据目录失败: {}", e))?;

    // 弹出文件选择对话框
    let (sender, receiver) = std::sync::mpsc::channel();
    app.dialog()
        .file()
        .add_filter("ZIP 文件", &["zip"])
        .pick_file(move |file_path| {
            let _ = sender.send(file_path);
        });

    let file_path = receiver.recv()
        .map_err(|_| "文件对话框取消或超时".to_string())?;

    let path = match file_path {
        Some(p) => p,
        None => return Err("未选择备份文件".to_string()),
    };

    let zip_path = std::path::PathBuf::from(path.to_string());
    local_backup::restore_from_local(&zip_path, &data_dir)
}
```

- [ ] **Step 2: 在 lib.rs 注册新命令**

在 `src-tauri/src/lib.rs` 的 `generate_handler!` 宏中，`preview::close_preview_window,` 之后（第 57 行后）添加：

```rust
            commands::test_s3_connection,
            commands::backup_to_s3,
            commands::list_s3_backups,
            commands::restore_from_s3,
            commands::backup_to_local,
            commands::restore_from_local,
```

- [ ] **Step 3: 验证编译通过**

Run: `cd /Users/lishaojie/PycharmProjects/pythonProject1/日常工具/byetype/src-tauri && cargo check`
Expected: 编译成功

- [ ] **Step 4: Commit**

```bash
cd /Users/lishaojie/PycharmProjects/pythonProject1/日常工具/byetype
git add src-tauri/src/commands.rs src-tauri/src/lib.rs
git commit -m "feat(backup): 添加 6 个备份恢复 Tauri 命令"
```

---

### Task 6: 添加前端 TypeScript 类型和 API 函数

**Files:**
- Modify: `src/core/types.ts`
- Modify: `src/lib/tauri-api.ts`

**Interfaces:**
- Produces: `S3Config`, `BackupConfig`, `BackupEntry` TypeScript interfaces
- Produces: 6 API functions — `testS3Connection`, `backupToS3`, `listS3Backups`, `restoreFromS3`, `backupToLocal`, `restoreFromLocal`

- [ ] **Step 1: 在 types.ts 添加 S3Config, BackupConfig, BackupEntry 类型**

在 `src/core/types.ts` 的 `AppConfig` interface 之前（第 97 行之前）添加：

```typescript
export interface S3Config {
  endpoint: string
  region: string
  bucket: string
  accessKey: string
  secretKey: string
  prefix: string
}

export interface BackupConfig {
  s3: S3Config
}

export interface BackupEntry {
  key: string
  size: number
  lastModified: string
}
```

- [ ] **Step 2: 在 AppConfig 中添加 backup 字段**

修改 `src/core/types.ts` 的 `AppConfig` interface（第 97-104 行）：

```typescript
export interface AppConfig {
  general: GeneralConfig
  models: ModelsConfig
  transcribe: TranscribeConfig
  voiceTemplates: VoiceTemplatesConfig
  extract: ExtractConfig
  advanced: AdvancedConfig
  backup: BackupConfig
}
```

- [ ] **Step 3: 在 tauri-api.ts 添加 backup API 函数**

在 `src/lib/tauri-api.ts` 文件末尾添加：

```typescript
// ==================== Backup ====================

export async function testS3Connection(): Promise<void> {
  return invoke<void>('test_s3_connection')
}

export async function backupToS3(): Promise<string> {
  return invoke<string>('backup_to_s3')
}

export async function listS3Backups(): Promise<BackupEntry[]> {
  return invoke<BackupEntry[]>('list_s3_backups')
}

export async function restoreFromS3(objectKey: string): Promise<void> {
  return invoke<void>('restore_from_s3', { objectKey })
}

export async function backupToLocal(): Promise<string> {
  return invoke<string>('backup_to_local')
}

export async function restoreFromLocal(): Promise<void> {
  return invoke<void>('restore_from_local')
}
```

同时在文件顶部的 import 中添加 `BackupEntry` 类型：

修改第 5 行：
```typescript
import type { AppConfig, AudioDevice, UpdateInfo, BackupEntry } from '../core/types'
```

- [ ] **Step 4: 验证前端编译通过**

Run: `cd /Users/lishaojie/PycharmProjects/pythonProject1/日常工具/byetype && npx tsc --noEmit`
Expected: 无类型错误

- [ ] **Step 5: Commit**

```bash
cd /Users/lishaojie/PycharmProjects/pythonProject1/日常工具/byetype
git add src/core/types.ts src/lib/tauri-api.ts
git commit -m "feat(backup): 添加前端 backup 类型和 API 函数"
```

---

### Task 7: 实现备份与恢复设置页 UI

**Files:**
- Create: `src/views/settings/tabs/BackupTab.tsx`
- Modify: `src/views/settings/App.tsx`

**Interfaces:**
- Consumes: `AppConfig`, `onSave` from App.tsx props, backup API functions from tauri-api.ts
- Produces: `BackupTab` React component

- [ ] **Step 1: 创建 BackupTab.tsx**

创建 `src/views/settings/tabs/BackupTab.tsx`：

```tsx
import React, { useState, useEffect, useCallback } from 'react'
import { SettingGroup } from '../components/SettingGroup'
import { SettingRow } from '../components/SettingRow'
import type { AppConfig, BackupEntry } from '../../../core/types'
import {
  testS3Connection,
  backupToS3,
  listS3Backups,
  restoreFromS3,
  backupToLocal,
  restoreFromLocal,
} from '../../../lib/tauri-api'

interface Props {
  config: AppConfig
  onSave: (config: AppConfig) => void
}

export function BackupTab({ config, onSave }: Props) {
  const [testing, setTesting] = useState(false)
  const [testResult, setTestResult] = useState<{ ok: boolean; msg: string } | null>(null)
  const [s3Backing, setS3Backing] = useState(false)
  const [s3Restoring, setS3Restoring] = useState(false)
  const [localBacking, setLocalBacking] = useState(false)
  const [localRestoring, setLocalRestoring] = useState(false)
  const [backups, setBackups] = useState<BackupEntry[]>([])
  const [loadingBackups, setLoadingBackups] = useState(false)
  const [message, setMessage] = useState<{ type: 'success' | 'error'; text: string } | null>(null)

  const s3 = config.backup.s3

  const updateS3 = (changes: Partial<typeof s3>) => {
    onSave({ ...config, backup: { ...config.backup, s3: { ...s3, ...changes } } })
  }

  const showMessage = (type: 'success' | 'error', text: string) => {
    setMessage({ type, text })
    setTimeout(() => setMessage(null), 5000)
  }

  const handleTestConnection = useCallback(async () => {
    setTesting(true)
    setTestResult(null)
    try {
      await testS3Connection()
      setTestResult({ ok: true, msg: '连接成功' })
    } catch (e: any) {
      setTestResult({ ok: false, msg: String(e) })
    } finally {
      setTesting(false)
    }
  }, [])

  const handleBackupToS3 = useCallback(async () => {
    setS3Backing(true)
    try {
      const key = await backupToS3()
      showMessage('success', `备份成功：${key}`)
      // 刷新列表
      await refreshBackups()
    } catch (e: any) {
      showMessage('error', `备份失败：${e}`)
    } finally {
      setS3Backing(false)
    }
  }, [])

  const handleRestoreFromS3 = useCallback(async (key: string) => {
    if (!confirm(`确认从 ${key} 恢复？当前配置将被覆盖，恢复后需要重启应用。`)) return
    setS3Restoring(true)
    try {
      await restoreFromS3(key)
      showMessage('success', '恢复成功，请重启应用使配置生效')
    } catch (e: any) {
      showMessage('error', `恢复失败：${e}`)
    } finally {
      setS3Restoring(false)
    }
  }, [])

  const handleBackupToLocal = useCallback(async () => {
    setLocalBacking(true)
    try {
      const path = await backupToLocal()
      showMessage('success', `备份已保存：${path}`)
    } catch (e: any) {
      if (String(e).includes('未选择')) return
      showMessage('error', `备份失败：${e}`)
    } finally {
      setLocalBacking(false)
    }
  }, [])

  const handleRestoreFromLocal = useCallback(async () => {
    if (!confirm('确认从本地文件恢复？当前配置将被覆盖，恢复后需要重启应用。')) return
    setLocalRestoring(true)
    try {
      await restoreFromLocal()
      showMessage('success', '恢复成功，请重启应用使配置生效')
    } catch (e: any) {
      if (String(e).includes('未选择')) return
      showMessage('error', `恢复失败：${e}`)
    } finally {
      setLocalRestoring(false)
    }
  }, [])

  const refreshBackups = useCallback(async () => {
    setLoadingBackups(true)
    try {
      const list = await listS3Backups()
      setBackups(list)
    } catch (e: any) {
      setBackups([])
    } finally {
      setLoadingBackups(false)
    }
  }, [])

  useEffect(() => {
    if (s3.bucket) {
      refreshBackups()
    }
  }, [s3.bucket])

  const formatSize = (bytes: number) => {
    if (bytes < 1024) return `${bytes} B`
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`
    return `${(bytes / 1024 / 1024).toFixed(1)} MB`
  }

  const formatDate = (s: string) => {
    try {
      return new Date(s).toLocaleString('zh-CN')
    } catch {
      return s
    }
  }

  return (
    <div>
      <h2>备份与恢复</h2>

      {message && (
        <div style={{
          padding: '8px 12px',
          marginBottom: 12,
          borderRadius: 6,
          background: message.type === 'success' ? '#d4edda' : '#f8d7da',
          color: message.type === 'success' ? '#155724' : '#721c24',
          fontSize: 13,
        }}>
          {message.text}
        </div>
      )}

      <SettingGroup title="S3 兼容存储配置">
        <SettingRow label="Endpoint" description="留空则使用 AWS S3 标准地址">
          <input
            type="text"
            value={s3.endpoint}
            onChange={e => updateS3({ endpoint: e.target.value })}
            placeholder="https://s3.amazonaws.com"
            style={{ width: 240 }}
          />
        </SettingRow>
        <SettingRow label="Region">
          <input
            type="text"
            value={s3.region}
            onChange={e => updateS3({ region: e.target.value })}
            placeholder="us-east-1"
            style={{ width: 240 }}
          />
        </SettingRow>
        <SettingRow label="Bucket">
          <input
            type="text"
            value={s3.bucket}
            onChange={e => updateS3({ bucket: e.target.value })}
            placeholder="my-backup-bucket"
            style={{ width: 240 }}
          />
        </SettingRow>
        <SettingRow label="Access Key">
          <input
            type="text"
            value={s3.accessKey}
            onChange={e => updateS3({ accessKey: e.target.value })}
            placeholder="AKIAXXXXX"
            style={{ width: 240 }}
          />
        </SettingRow>
        <SettingRow label="Secret Key">
          <input
            type="password"
            value={s3.secretKey}
            onChange={e => updateS3({ secretKey: e.target.value })}
            placeholder="******"
            style={{ width: 240 }}
          />
        </SettingRow>
        <SettingRow label="路径前缀" description="S3 对象 key 的前缀">
          <input
            type="text"
            value={s3.prefix}
            onChange={e => updateS3({ prefix: e.target.value })}
            placeholder="byetype/backups"
            style={{ width: 240 }}
          />
        </SettingRow>
        <SettingRow label="连接测试">
          <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
            <button onClick={handleTestConnection} disabled={testing} style={{ padding: '4px 12px' }}>
              {testing ? '测试中...' : '测试连接'}
            </button>
            {testResult && (
              <span style={{
                fontSize: 12,
                color: testResult.ok ? '#155724' : '#721c24',
              }}>
                {testResult.ok ? '✓' : '✗'} {testResult.msg}
              </span>
            )}
          </div>
        </SettingRow>
      </SettingGroup>

      <SettingGroup title="S3 备份">
        <SettingRow label="立即备份到 S3" description="将配置和提示词打包上传到 S3">
          <button onClick={handleBackupToS3} disabled={s3Backing || !s3.bucket} style={{ padding: '4px 12px' }}>
            {s3Backing ? '备份中...' : '备份到 S3'}
          </button>
        </SettingRow>
        <SettingRow label="从 S3 恢复" description="选择一个备份恢复">
          <button onClick={refreshBackups} disabled={loadingBackups || !s3.bucket} style={{ padding: '4px 12px' }}>
            {loadingBackups ? '加载中...' : '刷新列表'}
          </button>
        </SettingRow>
        {backups.length > 0 && (
          <div style={{ marginTop: 8 }}>
            <table style={{ width: '100%', fontSize: 12, borderCollapse: 'collapse' }}>
              <thead>
                <tr style={{ borderBottom: '1px solid #e0e0e0' }}>
                  <th style={{ textAlign: 'left', padding: '4px 8px' }}>备份文件</th>
                  <th style={{ textAlign: 'left', padding: '4px 8px' }}>大小</th>
                  <th style={{ textAlign: 'left', padding: '4px 8px' }}>时间</th>
                  <th style={{ padding: '4px 8px' }}>操作</th>
                </tr>
              </thead>
              <tbody>
                {backups.map(b => (
                  <tr key={b.key} style={{ borderBottom: '1px solid #f0f0f0' }}>
                    <td style={{ padding: '4px 8px' }}>{b.key.split('/').pop()}</td>
                    <td style={{ padding: '4px 8px' }}>{formatSize(b.size)}</td>
                    <td style={{ padding: '4px 8px' }}>{formatDate(b.lastModified)}</td>
                    <td style={{ padding: '4px 8px' }}>
                      <button
                        onClick={() => handleRestoreFromS3(b.key)}
                        disabled={s3Restoring}
                        style={{ padding: '2px 8px', fontSize: 12 }}
                      >
                        恢复
                      </button>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </SettingGroup>

      <SettingGroup title="本地备份">
        <SettingRow label="备份到本地" description="选择保存位置，导出 zip 备份文件">
          <button onClick={handleBackupToLocal} disabled={localBacking} style={{ padding: '4px 12px' }}>
            {localBacking ? '备份中...' : '备份到本地'}
          </button>
        </SettingRow>
        <SettingRow label="从本地恢复" description="选择 zip 备份文件恢复配置">
          <button onClick={handleRestoreFromLocal} disabled={localRestoring} style={{ padding: '4px 12px' }}>
            {localRestoring ? '恢复中...' : '从本地恢复'}
          </button>
        </SettingRow>
      </SettingGroup>
    </div>
  )
}
```

- [ ] **Step 2: 在 App.tsx 注册 backup tab**

在 `src/views/settings/App.tsx` 中：

1. 添加 import（第 9 行 `import { ExtractPromptsTab } from './tabs/ExtractPromptsTab'` 之后）：

```typescript
import { BackupTab } from './tabs/BackupTab'
```

2. 在 `TABS` 数组中，`{ type: 'divider' }` 之前（第 29 行之前）添加：

```typescript
  { type: 'tab', id: 'backup', label: '备份与恢复' },
```

3. 在条件渲染区域，`{activeTab === 'about' && ...}` 之前（第 193 行之前）添加：

```tsx
        {activeTab === 'backup' && config && <BackupTab config={config} onSave={handleSave} />}
```

- [ ] **Step 3: 验证前端编译通过**

Run: `cd /Users/lishaojie/PycharmProjects/pythonProject1/日常工具/byetype && npx tsc --noEmit`
Expected: 无类型错误

- [ ] **Step 4: Commit**

```bash
cd /Users/lishaojie/PycharmProjects/pythonProject1/日常工具/byetype
git add src/views/settings/tabs/BackupTab.tsx src/views/settings/App.tsx
git commit -m "feat(backup): 实现备份与恢复设置页 UI"
```

---

### Task 8: 端到端验证和修复

**Files:**
- May modify: any files from previous tasks if issues found

- [ ] **Step 1: 启动开发环境验证**

Run: `cd /Users/lishaojie/PycharmProjects/pythonProject1/日常工具/byetype && npm run tauri dev`
Expected: 应用正常启动，设置页出现「备份与恢复」tab

- [ ] **Step 2: 验证 UI 展示**

打开设置 → 备份与恢复 tab，确认：
- S3 配置区（endpoint, region, bucket, access key, secret key, prefix）显示正常
- 测试连接按钮可见
- S3 备份/恢复按钮可见
- 本地备份/恢复按钮可见

- [ ] **Step 3: 验证本地备份功能**

1. 点击「备份到本地」
2. 文件对话框弹出，选择保存位置
3. 确认生成 zip 文件
4. 用解压软件打开 zip，确认包含 config.json 和 prompts/ 目录

- [ ] **Step 4: 验证本地恢复功能**

1. 点击「从本地恢复」
2. 选择刚才备份的 zip 文件
3. 确认恢复成功提示

- [ ] **Step 5: 验证 S3 配置保存**

1. 填入 S3 配置（endpoint, region, bucket, access key, secret key）
2. 关闭设置窗口
3. 重新打开设置 → 备份与恢复，确认配置已保存

- [ ] **Step 6: 修复发现的问题**

如果有任何功能异常，修复后提交：

```bash
cd /Users/lishaojie/PycharmProjects/pythonProject1/日常工具/byetype
git add -A
git commit -m "fix(backup): 修复端到端测试发现的问题"
```

- [ ] **Step 7: 最终 Commit**

确保所有更改已提交：

```bash
cd /Users/lishaojie/PycharmProjects/pythonProject1/日常工具/byetype
git status
```
Expected: clean working tree
