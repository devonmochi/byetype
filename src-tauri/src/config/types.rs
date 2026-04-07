use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppConfig {
    pub general: GeneralConfig,
    pub models: ModelsConfig,
    pub transcribe: TranscribeConfig,
    pub optimize: OptimizeConfig,
    #[serde(default)]
    pub extract: ExtractConfig,
    pub advanced: AdvancedConfig,
}

fn default_true() -> bool {
    true
}

fn default_max_recording_seconds() -> u32 {
    180
}

fn default_microphone() -> String {
    "system-default".to_string()
}

fn default_extract_shortcut() -> String {
    "F6".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GeneralConfig {
    pub shortcut: String,
    pub launch_at_login: bool,
    pub theme: String,
    #[serde(default = "default_max_recording_seconds")]
    pub max_recording_seconds: u32,
    #[serde(default = "default_microphone")]
    pub microphone: String,
    #[serde(default = "default_extract_shortcut")]
    pub extract_shortcut: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelsConfig {
    pub builtin_api_keys: BuiltinApiKeys,
    #[serde(default)]
    pub custom: Vec<CustomModelEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BuiltinApiKeys {
    pub gemini: String,
    #[serde(default)]
    pub deepseek: String,
    #[serde(default)]
    pub dashscope: String,
    #[serde(default)]
    pub openrouter: String,
    #[serde(default)]
    pub mimo: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomModelEntry {
    pub id: String,
    pub provider: String,
    pub model: String,
    pub protocol: String,
    pub base_url: String,
    pub api_key: String,
    pub supports_audio: bool,
    pub supports_text: bool,
    #[serde(default = "default_true")]
    pub supports_vision: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThinkingConfig {
    pub enabled: bool,
    pub budget: u32,
    pub level: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptsConfig {
    pub agent: String,
    pub rules: String,
    pub vocabulary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TranscribeConfig {
    pub model_id: String,
    pub thinking: ThinkingConfig,
    pub prompts: PromptsConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OptimizeConfig {
    pub enabled: bool,
    pub model_id: String,
    pub thinking: ThinkingConfig,
    pub prompt: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtractConfig {
    pub model_id: Option<String>,
    pub thinking: Option<ThinkingConfig>,
    pub prompt: String,
}

impl Default for ExtractConfig {
    fn default() -> Self {
        Self {
            model_id: None,
            thinking: None,
            prompt: String::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdvancedConfig {
    pub transcribe_timeout: u32,
    pub optimize_timeout: u32,
    pub max_retries: u32,
    pub max_parallel: u32,
    pub proxy_url: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            general: GeneralConfig {
                shortcut: "F4".to_string(),
                launch_at_login: false,
                theme: "system".to_string(),
                max_recording_seconds: 180,
                microphone: "system-default".to_string(),
                extract_shortcut: "F6".to_string(),
            },
            models: ModelsConfig {
                builtin_api_keys: BuiltinApiKeys {
                    gemini: String::new(),
                    deepseek: String::new(),
                    dashscope: String::new(),
                    openrouter: String::new(),
                    mimo: String::new(),
                },
                custom: Vec::new(),
            },
            transcribe: TranscribeConfig {
                model_id: "builtin-gemini-3-flash".to_string(),
                thinking: ThinkingConfig {
                    enabled: false,
                    budget: 1024,
                    level: "LOW".to_string(),
                },
                prompts: PromptsConfig {
                    agent: String::new(),
                    rules: String::new(),
                    vocabulary: String::new(),
                },
            },
            optimize: OptimizeConfig {
                enabled: false,
                model_id: String::new(),
                thinking: ThinkingConfig {
                    enabled: false,
                    budget: 1024,
                    level: "LOW".to_string(),
                },
                prompt: String::new(),
            },
            extract: ExtractConfig::default(),
            advanced: AdvancedConfig {
                transcribe_timeout: 10,
                optimize_timeout: 10,
                max_retries: 3,
                max_parallel: 3,
                proxy_url: String::new(),
            },
        }
    }
}
