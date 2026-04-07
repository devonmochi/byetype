use crate::config::types::AppConfig;

pub struct BuiltinModel {
    pub id: &'static str,
    pub provider: &'static str,
    pub model: &'static str,
    pub protocol: &'static str,
    pub base_url: &'static str,
    pub supports_audio: bool,
    pub supports_text: bool,
    pub supports_vision: bool,
}

pub static BUILTIN_MODELS: &[BuiltinModel] = &[
    BuiltinModel {
        id: "builtin-qwen-omni-plus",
        provider: "阿里云百炼",
        model: "qwen3.5-omni-plus",
        protocol: "qwen-omni",
        base_url: "https://dashscope.aliyuncs.com/compatible-mode/v1",
        supports_audio: true,
        supports_text: true,
        supports_vision: true,
    },
    BuiltinModel {
        id: "builtin-qwen-omni-flash",
        provider: "阿里云百炼",
        model: "qwen3.5-omni-flash",
        protocol: "qwen-omni",
        base_url: "https://dashscope.aliyuncs.com/compatible-mode/v1",
        supports_audio: true,
        supports_text: true,
        supports_vision: true,
    },
    BuiltinModel {
        id: "builtin-longcat-omni",
        provider: "LongCat",
        model: "LongCat-Flash-Omni-2603",
        protocol: "longcat",
        base_url: "https://api.longcat.chat/openai/v1",
        supports_audio: true,
        supports_text: true,
        supports_vision: true,
    },
    BuiltinModel {
        id: "builtin-gemini-3-flash",
        provider: "Google Gemini",
        model: "gemini-3-flash-preview",
        protocol: "gemini",
        base_url: "https://generativelanguage.googleapis.com",
        supports_audio: true,
        supports_text: true,
        supports_vision: true,
    },
    BuiltinModel {
        id: "builtin-gemini-3.1-flash-lite",
        provider: "Google Gemini",
        model: "gemini-3.1-flash-lite-preview",
        protocol: "gemini",
        base_url: "https://generativelanguage.googleapis.com",
        supports_audio: true,
        supports_text: true,
        supports_vision: true,
    },
    BuiltinModel {
        id: "builtin-deepseek-chat",
        provider: "DeepSeek",
        model: "deepseek-chat",
        protocol: "openai-compat",
        base_url: "https://api.deepseek.com/v1",
        supports_audio: false,
        supports_text: true,
        supports_vision: false,
    },
    BuiltinModel {
        id: "builtin-or-qwen3.6-plus-free",
        provider: "OpenRouter",
        model: "qwen/qwen3.6-plus:free",
        protocol: "openai-compat",
        base_url: "https://openrouter.ai/api/v1",
        supports_audio: false,
        supports_text: true,
        supports_vision: true,
    },
    BuiltinModel {
        id: "builtin-or-gemini-3-flash",
        provider: "OpenRouter",
        model: "google/gemini-3-flash-preview",
        protocol: "openai-compat",
        base_url: "https://openrouter.ai/api/v1",
        supports_audio: true,
        supports_text: true,
        supports_vision: true,
    },
    BuiltinModel {
        id: "builtin-or-gemini-3.1-flash-lite",
        provider: "OpenRouter",
        model: "google/gemini-3.1-flash-lite-preview",
        protocol: "openai-compat",
        base_url: "https://openrouter.ai/api/v1",
        supports_audio: true,
        supports_text: true,
        supports_vision: true,
    },
    BuiltinModel {
        id: "builtin-mimo-v2-omni",
        provider: "小米 MiMo",
        model: "mimo-v2-omni",
        protocol: "mimo",
        base_url: "https://api.xiaomimimo.com/v1",
        supports_audio: true,
        supports_text: true,
        supports_vision: true,
    },
];

pub struct ResolvedModel {
    pub protocol: String,
    pub base_url: String,
    pub model: String,
    pub api_key: String,
}

pub fn resolve_model(config: &AppConfig, model_id: &str) -> Result<ResolvedModel, String> {
    if let Some(builtin) = BUILTIN_MODELS.iter().find(|m| m.id == model_id) {
        let api_key = if model_id.starts_with("builtin-or-") {
            &config.models.builtin_api_keys.openrouter
        } else {
            match builtin.protocol {
                "gemini" => &config.models.builtin_api_keys.gemini,
                "openai-compat" => &config.models.builtin_api_keys.deepseek,
                "qwen-omni" => &config.models.builtin_api_keys.dashscope,
                "mimo" => &config.models.builtin_api_keys.mimo,
                "longcat" => &config.models.builtin_api_keys.longcat,
                _ => return Err(format!("Unknown protocol for builtin model: {}", model_id)),
            }
        };
        return Ok(ResolvedModel {
            protocol: builtin.protocol.to_string(),
            base_url: builtin.base_url.to_string(),
            model: builtin.model.to_string(),
            api_key: api_key.clone(),
        });
    }

    if let Some(custom) = config.models.custom.iter().find(|m| m.id == model_id) {
        return Ok(ResolvedModel {
            protocol: custom.protocol.clone(),
            base_url: custom.base_url.clone(),
            model: custom.model.clone(),
            api_key: custom.api_key.clone(),
        });
    }

    Err(format!("Model not found: {}", model_id))
}
