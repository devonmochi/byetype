pub mod types;
mod migration;

use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use types::AppConfig;

pub struct ConfigManager {
    config_path: PathBuf,
    config: Mutex<AppConfig>,
}

impl ConfigManager {
    pub fn new(config_dir: PathBuf) -> Self {
        fs::create_dir_all(&config_dir).ok();
        let config_path = config_dir.join("config.json");

        // 迁移：旧版 config.json 在 dirs::config_dir()/byetype/，新版统一到 app_data_dir
        if !config_path.exists() {
            let old_dir = dirs::config_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join("byetype");
            let old_path = old_dir.join("config.json");
            if old_path.exists() {
                fs::copy(&old_path, &config_path).ok();
            }
        }

        let config = Self::load(&config_path);
        Self {
            config_path,
            config: Mutex::new(config),
        }
    }

    fn load(path: &PathBuf) -> AppConfig {
        if path.exists() {
            match fs::read_to_string(path) {
                Ok(raw) => {
                    // Try to parse as Value first for migration
                    match serde_json::from_str::<serde_json::Value>(&raw) {
                        Ok(mut json_value) => {
                            if migration::migrate_if_needed(&mut json_value) {
                                // Migration occurred, save the migrated config back to disk
                                if let Ok(migrated_json) = serde_json::to_string_pretty(&json_value) {
                                    // 原子写入：先写临时文件再 rename，避免写盘中途损坏 config.json
                                    let tmp = path.with_extension("json.tmp");
                                    // 以 0o600 权限直接创建临时文件，避免在权限收紧前的时间窗口内泄露敏感凭证
                                    #[cfg(unix)]
                                    let write_ok = {
                                        use std::os::unix::fs::OpenOptionsExt;
                                        use std::io::Write;
                                        std::fs::OpenOptions::new()
                                            .write(true)
                                            .create(true)
                                            .truncate(true)
                                            .mode(0o600)
                                            .open(&tmp)
                                            .and_then(|mut f| f.write_all(migrated_json.as_bytes()))
                                            .is_ok()
                                    };
                                    #[cfg(not(unix))]
                                    let write_ok = fs::write(&tmp, &migrated_json).is_ok();
                                    if write_ok {
                                        if fs::rename(&tmp, path).is_err() {
                                            // rename 失败时清理残留临时文件，避免敏感信息残留
                                            let _ = fs::remove_file(&tmp);
                                        } else {
                                            // 收紧文件权限为仅属主可读写，防止敏感凭证泄露
                                            #[cfg(unix)]
                                            {
                                                use std::os::unix::fs::PermissionsExt;
                                                if let Ok(meta) = fs::metadata(path) {
                                                    let mut perm = meta.permissions();
                                                    perm.set_mode(0o600);
                                                    let _ = fs::set_permissions(path, perm);
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            // Deserialize from the (possibly migrated) Value
                            match serde_json::from_value::<AppConfig>(json_value) {
                                Ok(cfg) => cfg,
                                Err(e) => {
                                    eprintln!("[config] 反序列化失败，回退默认值: {e}");
                                    AppConfig::default()
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!("[config] JSON 解析失败，回退默认值: {e}");
                            AppConfig::default()
                        }
                    }
                }
                Err(e) => {
                    eprintln!("[config] 配置文件读取失败，回退默认值: {e}");
                    AppConfig::default()
                }
            }
        } else {
            AppConfig::default()
        }
    }

    pub fn get(&self) -> AppConfig {
        self.config.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }

    pub fn update(&self, new_config: AppConfig) -> Result<(), String> {
        // 先序列化，落盘与内存更新都在同一锁临界区内完成：
        // 既避免写盘失败时内存与磁盘状态不一致，也防止并发 update() 争用同一临时文件导致 config.json 损坏
        let json = serde_json::to_string_pretty(&new_config).map_err(|e| e.to_string())?;
        let mut config = self.config.lock().unwrap_or_else(|e| e.into_inner());
        // 原子写入：先写临时文件再 rename，避免写盘中途崩溃导致 config.json 被截断损坏
        let tmp = self.config_path.with_extension("json.tmp");
        // 以 0o600 权限直接创建临时文件，从源头避免在权限收紧前的时间窗口内泄露 API Key 等敏感凭证
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            let mut file = std::fs::OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .mode(0o600)
                .open(&tmp)
                .map_err(|e| e.to_string())?;
            use std::io::Write;
            file.write_all(json.as_bytes()).map_err(|e| e.to_string())?;
        }
        #[cfg(not(unix))]
        {
            fs::write(&tmp, &json).map_err(|e| e.to_string())?;
        }
        if let Err(e) = fs::rename(&tmp, &self.config_path) {
            // rename 失败时清理残留临时文件，避免敏感信息残留
            let _ = fs::remove_file(&tmp);
            return Err(e.to_string());
        }
        // 收紧文件权限为仅属主可读写，防止同机其他本地用户读取 API Key 等敏感凭证
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Ok(meta) = fs::metadata(&self.config_path) {
                let mut perm = meta.permissions();
                perm.set_mode(0o600);
                let _ = fs::set_permissions(&self.config_path, perm);
            }
        }
        *config = new_config;
        Ok(())
    }
}
