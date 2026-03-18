use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppConfig {
    pub general: GeneralConfig,
    pub transcribe: TranscribeConfig,
    pub optimize: OptimizeConfig,
    pub advanced: AdvancedConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GeneralConfig {
    pub shortcut: String,
    pub launch_at_login: bool,
    pub theme: String,
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
    pub model: String,
    pub gemini_api_key: String,
    pub qwen_api_key: String,
    pub thinking: ThinkingConfig,
    pub prompts: PromptsConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenAICompatConfig {
    pub provider_name: String,
    pub base_url: String,
    pub model: String,
    pub api_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OptimizeConfig {
    pub enabled: bool,
    #[serde(rename = "type")]
    pub optimize_type: String,
    pub openai_compat: OpenAICompatConfig,
    pub gemini_model: String,
    pub thinking: ThinkingConfig,
    pub prompt: String,
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
            },
            transcribe: TranscribeConfig {
                model: "gemini-3-flash-preview".to_string(),
                gemini_api_key: String::new(),
                qwen_api_key: String::new(),
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
                optimize_type: "openai-compat".to_string(),
                openai_compat: OpenAICompatConfig {
                    provider_name: String::new(),
                    base_url: String::new(),
                    model: String::new(),
                    api_key: String::new(),
                },
                gemini_model: "gemini-3-flash-preview".to_string(),
                thinking: ThinkingConfig {
                    enabled: false,
                    budget: 1024,
                    level: "LOW".to_string(),
                },
                prompt: String::new(),
            },
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
