use serde::{Deserialize, Serialize};
use s3::{AddressingStyle, Auth, Client, Credentials};
use crate::config::types::S3Config;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupEntry {
    pub key: String,
    pub size: u64,
    pub last_modified: String,
}

fn create_client(config: &S3Config) -> Result<Client, String> {
    if config.bucket.is_empty() {
        return Err("未配置 bucket".to_string());
    }
    if config.access_key.is_empty() || config.secret_key.is_empty() {
        return Err("未配置 Access Key 或 Secret Key".to_string());
    }
    if config.region.is_empty() {
        return Err("未配置 region".to_string());
    }

    // 构造 endpoint：若用户未填 endpoint，按 AWS 规则拼装默认地址
    let endpoint = if config.endpoint.is_empty() {
        if config.region == "us-east-1" {
            "https://s3.amazonaws.com".to_string()
        } else if config.region.starts_with("cn-") {
            format!("https://s3.{}.amazonaws.com.cn", config.region)
        } else {
            format!("https://s3.{}.amazonaws.com", config.region)
        }
    } else if config.endpoint.starts_with("http://")
        || config.endpoint.starts_with("https://")
    {
        config.endpoint.clone()
    } else {
        format!("https://{}", config.endpoint)
    };

    let credentials = Credentials::new(&config.access_key, &config.secret_key)
        .map_err(|e| format!("凭证创建失败: {}", e))?;

    let mut builder = Client::builder(&endpoint)
        .map_err(|e| format!("创建 S3 客户端失败: {}", e))?
        .region(&config.region)
        .auth(Auth::Static(credentials));

    // 自定义 endpoint 使用 path style（兼容 MinIO / R2 等）
    if !config.endpoint.is_empty() {
        builder = builder.addressing_style(AddressingStyle::Path);
    }

    builder
        .build()
        .map_err(|e| format!("创建 S3 客户端失败: {}", e))
}

pub async fn test_connection(config: &S3Config) -> Result<(), String> {
    let client = create_client(config)?;
    client
        .buckets()
        .head(&config.bucket)
        .send()
        .await
        .map_err(|e| format!("连接 S3 失败: {}", e))?;
    Ok(())
}

pub async fn upload(config: &S3Config, data: Vec<u8>, key: String) -> Result<String, String> {
    let client = create_client(config)?;
    client
        .objects()
        .put(&config.bucket, &key)
        .body_bytes(data)
        .send()
        .await
        .map_err(|e| format!("上传 S3 失败: {}", e))?;
    Ok(key)
}

pub async fn download(config: &S3Config, key: &str) -> Result<Vec<u8>, String> {
    let client = create_client(config)?;
    let output = client
        .objects()
        .get(&config.bucket, key)
        .send()
        .await
        .map_err(|e| format!("从 S3 下载失败: {}", e))?;
    let bytes = output
        .bytes()
        .await
        .map_err(|e| format!("读取下载内容失败: {}", e))?;
    Ok(bytes.to_vec())
}

pub async fn list_backups(config: &S3Config) -> Result<Vec<BackupEntry>, String> {
    let client = create_client(config)?;
    let prefix = if config.prefix.is_empty() {
        "byetype/backups".to_string()
    } else {
        config.prefix.clone()
    };

    let output = client
        .objects()
        .list_v2(&config.bucket)
        .prefix(&prefix)
        .send()
        .await
        .map_err(|e| format!("列出 S3 备份失败: {}", e))?;

    let mut entries = Vec::new();
    for obj in output.contents {
        // 只返回 zip 文件
        if obj.key.ends_with(".zip") {
            entries.push(BackupEntry {
                key: obj.key,
                size: obj.size,
                last_modified: obj.last_modified.unwrap_or_default(),
            });
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
