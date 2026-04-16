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
                            serde_json::from_value(json_value).unwrap_or_default()
                        }
                        Err(_) => AppConfig::default(),
                    }
                }
                Err(_) => AppConfig::default(),
            }
        } else {
            AppConfig::default()
        }
    }

    pub fn get(&self) -> AppConfig {
        self.config.lock().unwrap().clone()
    }

    pub fn update(&self, new_config: AppConfig) -> Result<(), String> {
        let mut config = self.config.lock().unwrap();
        *config = new_config;
        let json = serde_json::to_string_pretty(&*config)
            .map_err(|e| e.to_string())?;
        fs::write(&self.config_path, json)
            .map_err(|e| e.to_string())?;
        Ok(())
    }
}
