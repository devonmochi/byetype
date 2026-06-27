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
                                    if fs::write(&tmp, &migrated_json).is_ok() {
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
        // 先序列化并落盘，成功后再更新内存，避免写盘失败时内存与磁盘状态不一致
        let json = serde_json::to_string_pretty(&new_config).map_err(|e| e.to_string())?;
        // 原子写入：先写临时文件再 rename，避免写盘中途崩溃导致 config.json 被截断损坏
        let tmp = self.config_path.with_extension("json.tmp");
        fs::write(&tmp, &json).map_err(|e| e.to_string())?;
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
        let mut config = self.config.lock().unwrap_or_else(|e| e.into_inner());
        *config = new_config;
        Ok(())
    }
}
