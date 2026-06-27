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
                                    fs::write(path, &migrated_json).ok();
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
        fs::write(&self.config_path, json).map_err(|e| e.to_string())?;
        let mut config = self.config.lock().unwrap_or_else(|e| e.into_inner());
        *config = new_config;
        Ok(())
    }
}
