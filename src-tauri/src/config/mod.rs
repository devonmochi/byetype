pub mod shortcut_config;
pub mod types;
mod migration;

use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use serde_json::Value;
use types::AppConfig;

use shortcut_config::{
    merge_config_preserving_shortcuts, migrate_shortcuts, sanitized_for_deserialize, schema_mode,
    set_field, ShortcutSchemaMode,
};

struct ConfigState {
    config: AppConfig,
    /// Latest raw JSON as persisted (or the best available view if a write
    /// failed). Preserves unknown fields and the shortcut schema marker.
    raw: Value,
    schema_mode: ShortcutSchemaMode,
    /// Human readable notes about the last shortcut migration; surfaced in the
    /// shortcut diagnostics log.
    migration_notes: Vec<String>,
}

pub struct ConfigManager {
    config_path: PathBuf,
    state: Mutex<ConfigState>,
}

fn is_windows() -> bool {
    cfg!(target_os = "windows")
}

impl ConfigManager {
    pub fn new(config_dir: PathBuf, legacy_config_dir: PathBuf) -> Self {
        fs::create_dir_all(&config_dir).ok();
        let config_path = config_dir.join("config.json");

        // 迁移：旧版 config.json 在系统配置目录的 byetype/ 下，新版统一到 app_data_dir
        if !config_path.exists() {
            let old_path = legacy_config_dir.join("byetype").join("config.json");
            if old_path.exists() {
                fs::copy(&old_path, &config_path).ok();
            }
        }

        let state = Self::load(&config_path);
        Self {
            config_path,
            state: Mutex::new(state),
        }
    }

    fn load(path: &PathBuf) -> ConfigState {
        if !path.exists() {
            // Fresh install: persist defaults so the shortcut schema marker and
            // platform default shortcuts exist on disk from the start.
            let config = AppConfig::default();
            let mut raw = serde_json::to_value(&config).unwrap_or_else(|_| serde_json::json!({}));
            if is_windows() {
                // Persist the Windows shortcut schema marker on a fresh install.
                shortcut_config::ensure_general_object(&mut raw).ok();
                migrate_shortcuts(&mut raw, true, true);
                atomic_write_value(path, &raw).ok();
            }
            return ConfigState {
                config,
                schema_mode: schema_mode(&raw),
                raw,
                migration_notes: vec!["新配置文件（无历史迁移）".to_string()],
            };
        }

        let raw_text = match fs::read_to_string(path) {
            Ok(text) => text,
            Err(e) => {
                eprintln!("[config] 配置文件读取失败，回退默认值: {e}");
                return Self::default_state();
            }
        };

        let mut raw: Value = match serde_json::from_str(&raw_text) {
            Ok(value) => value,
            Err(e) => {
                eprintln!("[config] JSON 解析失败，回退默认值: {e}");
                return Self::default_state();
            }
        };

        let original = raw.clone();
        let mode = schema_mode(&raw);
        let mut migration_notes = vec![format!(
            "磁盘原始: {}",
            shortcut_config::raw_shortcut_summary(&original)
        )];

        if mode == ShortcutSchemaMode::Supported {
            // 现有非快捷键迁移（模型 ID 等）；与本次快捷键需求无关但必须保留。
            let model_migrated = migration::migrate_if_needed(&mut raw);
            let schema_lt_1 = matches!(
                shortcut_config::parse_schema_version(&original),
                shortcut_config::RawSchemaVersion::Missing
                    | shortcut_config::RawSchemaVersion::Version(0)
            );
            let outcome = migrate_shortcuts(&mut raw, is_windows(), schema_lt_1);
            migration_notes.push(format!(
                "迁移: model={} changed={} f4_replaced={} f4_kept_conflict={} future={} schema_completed={}",
                model_migrated,
                outcome.changed,
                outcome.f4_replaced,
                outcome.f4_kept_conflict,
                outcome.future_mode,
                outcome.schema_completed
            ));
            migration_notes.extend(outcome.diagnostics.iter().cloned());
            if model_migrated || outcome.changed {
                // 原子写盘；只有成功后才把迁移视为完成（§15.6）。
                if let Err(e) = atomic_write_value(path, &raw) {
                    eprintln!("[config] 快捷键迁移写盘失败，将以旧配置运行并在下次启动重试: {e}");
                    migration_notes.push(format!("迁移写盘失败: {e}"));
                    raw = original;
                }
            }
        } else {
            migration_notes.push("进入 FutureSchemaMode：不自动迁移/写回".to_string());
        }
        migration_notes.push(format!(
            "迁移后: {}",
            shortcut_config::raw_shortcut_summary(&raw)
        ));

        let config = deserialize_config(&raw);
        ConfigState {
            config,
            schema_mode: mode,
            raw,
            migration_notes,
        }
    }

    fn default_state() -> ConfigState {
        let config = AppConfig::default();
        let raw = serde_json::to_value(&config).unwrap_or_else(|_| serde_json::json!({}));
        ConfigState {
            config,
            schema_mode: schema_mode(&raw),
            raw,
            migration_notes: vec!["配置读取/解析失败，使用默认值".to_string()],
        }
    }

    /// Notes captured while loading/migrating the shortcut config.
    pub fn migration_notes(&self) -> Vec<String> {
        self.state
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .migration_notes
            .clone()
    }

    pub fn get(&self) -> AppConfig {
        self.state
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .config
            .clone()
    }

    /// Latest raw JSON. Used by the shortcut layer to inspect the exact
    /// persisted representation of the four fields.
    #[allow(dead_code)]
    pub fn get_raw(&self) -> Value {
        self.state
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .raw
            .clone()
    }

    pub fn schema_mode(&self) -> ShortcutSchemaMode {
        self.state
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .schema_mode
    }

    /// Full config update for non-shortcut settings.
    ///
    /// The latest raw JSON is the base, so unknown fields survive. The four
    /// shortcut fields and the schema marker are always taken from the current
    /// raw state — shortcut changes must go through [`Self::commit_shortcut_patch`].
    pub fn update(&self, new_config: AppConfig) -> Result<(), String> {
        let incoming = serde_json::to_value(&new_config).map_err(|e| e.to_string())?;
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());

        let mut candidate = state.raw.clone();
        merge_config_preserving_shortcuts(&mut candidate, incoming);
        let view = deserialize_config(&candidate);

        atomic_write_value(&self.config_path, &candidate)?;
        state.raw = candidate;
        state.config = view;
        Ok(())
    }

    /// Persist a field-level shortcut patch on top of the latest raw JSON.
    ///
    /// This is the only path that may change shortcut values. It preserves all
    /// other fields, unknown keys and the (possibly higher) schema marker.
    pub fn commit_shortcut_patch(
        &self,
        patch: &[(crate::shortcut::model::ShortcutField, String)],
    ) -> Result<AppConfig, String> {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        let mut candidate = state.raw.clone();
        for (field, value) in patch {
            set_field(&mut candidate, *field, value)?;
        }
        let view = deserialize_config(&candidate);
        atomic_write_value(&self.config_path, &candidate).map_err(|e| {
            format!("CONFIG_PERSIST_FAILED: {}", e)
        })?;
        state.raw = candidate;
        state.config = view.clone();
        Ok(view)
    }
}

fn deserialize_config(raw: &Value) -> AppConfig {
    let sanitized = sanitized_for_deserialize(raw);
    match serde_json::from_value::<AppConfig>(sanitized) {
        Ok(config) => config,
        Err(e) => {
            eprintln!("[config] 反序列化失败，回退默认值: {e}");
            AppConfig::default()
        }
    }
}

/// Atomic write: write to a temporary file, then rename over the target.
/// Mirrors the project's existing safe-write behaviour (temp file + rename,
/// restrictive permissions on unix).
fn atomic_write_value(path: &PathBuf, value: &Value) -> Result<(), String> {
    let json = serde_json::to_string_pretty(value).map_err(|e| e.to_string())?;
    let tmp = path.with_extension("json.tmp");

    #[cfg(unix)]
    {
        use std::io::Write;
        use std::os::unix::fs::OpenOptionsExt;
        // 以 0o600 权限直接创建临时文件，避免在权限收紧前的时间窗口内泄露敏感凭证
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(&tmp)
            .map_err(|e| e.to_string())?;
        if let Err(e) = file.write_all(json.as_bytes()) {
            drop(file);
            let _ = fs::remove_file(&tmp);
            return Err(e.to_string());
        }
        drop(file);
    }
    #[cfg(not(unix))]
    {
        fs::write(&tmp, &json).map_err(|e| e.to_string())?;
    }

    if let Err(e) = fs::rename(&tmp, path) {
        let _ = fs::remove_file(&tmp);
        return Err(e.to_string());
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(meta) = fs::metadata(path) {
            let mut perm = meta.permissions();
            perm.set_mode(0o600);
            let _ = fs::set_permissions(path, perm);
        }
    }

    Ok(())
}
