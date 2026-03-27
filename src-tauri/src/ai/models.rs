use crate::config::types::AppConfig;

pub struct BuiltinModel {
    pub id: &'static str,
    pub provider: &'static str,
    pub model: &'static str,
    pub protocol: &'static str,
    pub base_url: &'static str,
    pub supports_audio: bool,
    pub supports_text: bool,
}

pub static BUILTIN_MODELS: &[BuiltinModel] = &[
    BuiltinModel {
        id: "builtin-gemini-3-flash",
        provider: "Google Gemini",
        model: "gemini-3-flash-preview",
        protocol: "gemini",
        base_url: "https://generativelanguage.googleapis.com",
        supports_audio: true,
        supports_text: true,
    },
    BuiltinModel {
        id: "builtin-gemini-3.1-flash-lite",
        provider: "Google Gemini",
        model: "gemini-3.1-flash-lite-preview",
        protocol: "gemini",
        base_url: "https://generativelanguage.googleapis.com",
        supports_audio: true,
        supports_text: true,
    },
    BuiltinModel {
        id: "builtin-qwen3-omni-flash",
        provider: "阿里云百炼",
        model: "qwen3-omni-flash",
        protocol: "openai-compat",
        base_url: "https://dashscope.aliyuncs.com/compatible-mode/v1",
        supports_audio: true,
        supports_text: false,
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
        let api_key = match builtin.protocol {
            "gemini" => &config.models.builtin_api_keys.gemini,
            "openai-compat" => &config.models.builtin_api_keys.qwen,
            _ => return Err(format!("Unknown protocol for builtin model: {}", model_id)),
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
