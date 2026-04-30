use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppConfig {
    pub general: GeneralConfig,
    pub models: ModelsConfig,
    pub transcribe: TranscribeConfig,
    #[serde(alias = "optimize")]
    pub voice_templates: VoiceTemplatesConfig,
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

fn default_shortcut2() -> String {
    String::new()
}

fn default_extract_shortcut2() -> String {
    String::new()
}

fn default_shortcut_template() -> String {
    "voice-optimize".to_string()
}

fn default_extract_template() -> String {
    "image-extract".to_string()
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
    #[serde(default = "default_shortcut2")]
    pub shortcut2: String,
    #[serde(default = "default_extract_shortcut2")]
    pub extract_shortcut2: String,
    #[serde(default = "default_shortcut_template")]
    pub shortcut_template: String,
    #[serde(default)]
    pub shortcut2_template: String,
    #[serde(default = "default_extract_template")]
    pub extract_shortcut_template: String,
    #[serde(default)]
    pub extract_shortcut2_template: String,
    #[serde(default)]
    pub shortcut_label: Option<String>,
    #[serde(default)]
    pub shortcut2_label: Option<String>,
    #[serde(default)]
    pub extract_shortcut_label: Option<String>,
    #[serde(default)]
    pub extract_shortcut2_label: Option<String>,
    #[serde(default)]
    pub ptt_mode: bool,
    #[serde(default = "default_true")]
    pub overwrite_clipboard: bool,
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
    #[serde(default)]
    pub longcat: String,
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
pub struct TemplateEntry {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub prompt: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoiceTemplatesConfig {
    pub model_id: String,
    pub thinking: ThinkingConfig,
    #[serde(default = "default_voice_templates")]
    pub templates: Vec<TemplateEntry>,
    /// DeepSeek 专用:reasoning_effort,取值 "high" / "max"。
    /// 仅在 model_id 指向 DeepSeek 且 thinking.enabled=true 时生效。
    #[serde(default)]
    pub deepseek_reasoning_effort: Option<String>,
}

fn default_voice_templates() -> Vec<TemplateEntry> {
    vec![
        TemplateEntry { id: "voice-optimize".to_string(), name: "自动换行".to_string(), prompt: String::new() },
        TemplateEntry { id: "voice-translate".to_string(), name: "翻译".to_string(), prompt: String::new() },
        TemplateEntry { id: "voice-custom".to_string(), name: "自定义".to_string(), prompt: String::new() },
    ]
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtractConfig {
    pub model_id: Option<String>,
    pub thinking: Option<ThinkingConfig>,
    pub prompt: String,
    #[serde(default = "default_image_templates")]
    pub templates: Vec<TemplateEntry>,
}

fn default_image_templates() -> Vec<TemplateEntry> {
    vec![
        TemplateEntry { id: "image-extract".to_string(), name: "文字识别".to_string(), prompt: String::new() },
        TemplateEntry { id: "image-translate".to_string(), name: "翻译".to_string(), prompt: String::new() },
        TemplateEntry { id: "image-custom".to_string(), name: "自定义".to_string(), prompt: String::new() },
    ]
}

impl Default for ExtractConfig {
    fn default() -> Self {
        Self {
            model_id: None,
            thinking: None,
            prompt: String::new(),
            templates: default_image_templates(),
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
    #[serde(default = "default_true")]
    pub proxy_enabled: bool,
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
                shortcut2: String::new(),
                extract_shortcut2: String::new(),
                shortcut_template: "voice-optimize".to_string(),
                shortcut2_template: "voice-translate".to_string(),
                extract_shortcut_template: "image-extract".to_string(),
                extract_shortcut2_template: "image-translate".to_string(),
                shortcut_label: None,
                shortcut2_label: None,
                extract_shortcut_label: None,
                extract_shortcut2_label: None,
                ptt_mode: false,
                overwrite_clipboard: true,
            },
            models: ModelsConfig {
                builtin_api_keys: BuiltinApiKeys {
                    gemini: String::new(),
                    deepseek: String::new(),
                    dashscope: String::new(),
                    openrouter: String::new(),
                    mimo: String::new(),
                    longcat: String::new(),
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
            voice_templates: VoiceTemplatesConfig {
                model_id: String::new(),
                thinking: ThinkingConfig {
                    enabled: false,
                    budget: 1024,
                    level: "LOW".to_string(),
                },
                templates: default_voice_templates(),
                deepseek_reasoning_effort: None,
            },
            extract: ExtractConfig::default(),
            advanced: AdvancedConfig {
                transcribe_timeout: 10,
                optimize_timeout: 10,
                max_retries: 3,
                max_parallel: 3,
                proxy_enabled: true,
                proxy_url: String::new(),
            },
        }
    }
}
