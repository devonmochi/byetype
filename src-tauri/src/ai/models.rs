use crate::config::types::{AppConfig, AudioInputMode};

pub struct BuiltinModel {
    pub id: &'static str,
    pub provider: &'static str,
    pub model: &'static str,
    pub protocol: &'static str,
    pub base_url: &'static str,
    pub supports_text: bool,
}

pub static BUILTIN_MODELS: &[BuiltinModel] = &[
    BuiltinModel {
        id: "builtin-qwen-omni-plus",
        provider: "阿里云百炼",
        model: "qwen3.5-omni-plus",
        protocol: "qwen-omni",
        base_url: "https://dashscope.aliyuncs.com/compatible-mode/v1",
        supports_text: true,
    },
    BuiltinModel {
        id: "builtin-qwen-omni-flash",
        provider: "阿里云百炼",
        model: "qwen3.5-omni-flash",
        protocol: "qwen-omni",
        base_url: "https://dashscope.aliyuncs.com/compatible-mode/v1",
        supports_text: true,
    },
    BuiltinModel {
        id: "builtin-gemini-3.8-flash",
        provider: "Google Gemini",
        model: "gemini-3.8-flash",
        protocol: "gemini",
        base_url: "https://generativelanguage.googleapis.com",
        supports_text: true,
    },
    BuiltinModel {
        id: "builtin-mimo-v2.5",
        provider: "XiaoMi",
        model: "mimo-v2.5",
        protocol: "mimo",
        base_url: "https://api.xiaomimimo.com/v1",
        supports_text: true,
    },
    BuiltinModel {
        id: "builtin-or-gemini-3.8-flash",
        provider: "OpenRouter",
        model: "google/gemini-3.8-flash",
        protocol: "openai-compat",
        base_url: "https://openrouter.ai/api/v1",
        supports_text: true,
    },
    BuiltinModel {
        id: "builtin-or-gemini-3.5-flash-lite",
        provider: "OpenRouter",
        model: "google/gemini-3.5-flash-lite",
        protocol: "openai-compat",
        base_url: "https://openrouter.ai/api/v1",
        supports_text: true,
    },
    BuiltinModel {
        id: "builtin-deepseek-flash",
        provider: "DeepSeek",
        model: "deepseek-flash",
        protocol: "openai-compat",
        base_url: "https://api.deepseek.com",
        supports_text: true,
    },
];

pub struct ResolvedModel {
    pub protocol: String,
    pub base_url: String,
    pub model: String,
    /// 服务商显示名（内置模型取预设名，自定义模型取用户填写名），用于用量统计展示。
    pub provider_label: String,
    pub api_key: String,
    pub audio_input_mode: AudioInputMode,
    pub chat_template_kwargs: Option<serde_json::Value>,
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
                _ => return Err(format!("Unknown protocol for builtin model: {}", model_id)),
            }
        };
        return Ok(ResolvedModel {
            protocol: builtin.protocol.to_string(),
            base_url: builtin.base_url.to_string(),
            model: builtin.model.to_string(),
            provider_label: builtin.provider.to_string(),
            api_key: api_key.clone(),
            audio_input_mode: AudioInputMode::InputAudio,
            chat_template_kwargs: None,
        });
    }

    if let Some(custom) = config.models.custom.iter().find(|m| m.id == model_id) {
        return Ok(ResolvedModel {
            protocol: custom.protocol.clone(),
            base_url: custom.base_url.clone(),
            model: custom.model.clone(),
            provider_label: custom.provider.clone(),
            api_key: custom.api_key.clone(),
            audio_input_mode: custom.audio_input_mode,
            chat_template_kwargs: Some(custom.chat_template_kwargs.clone()),
        });
    }

    Err(format!("Model not found: {}", model_id))
}

pub fn supports_text(config: &AppConfig, model_id: &str) -> Result<bool, String> {
    if let Some(builtin) = BUILTIN_MODELS.iter().find(|model| model.id == model_id) {
        return Ok(builtin.supports_text);
    }
    if let Some(custom) = config
        .models
        .custom
        .iter()
        .find(|model| model.id == model_id)
    {
        return Ok(custom.supports_text);
    }
    Err(format!("Model not found: {}", model_id))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::types::CustomModelEntry;

    #[test]
    fn offers_single_deepseek_model() {
        let deepseek: Vec<&BuiltinModel> = BUILTIN_MODELS
            .iter()
            .filter(|model| model.provider == "DeepSeek")
            .collect();

        assert_eq!(deepseek.len(), 1);
        assert_eq!(deepseek[0].id, "builtin-deepseek-flash");
        assert_eq!(deepseek[0].model, "deepseek-flash");
        assert!(deepseek[0].supports_text);
    }

    #[test]
    fn reports_custom_audio_only_model_as_not_text_capable() {
        let mut config = AppConfig::default();
        config.models.custom.push(CustomModelEntry {
            id: "audio-only".to_string(),
            provider: "custom".to_string(),
            model: "audio-only".to_string(),
            protocol: "openai-compat".to_string(),
            base_url: "https://example.com".to_string(),
            api_key: "test".to_string(),
            audio_input_mode: AudioInputMode::InputAudio,
            chat_template_kwargs: serde_json::json!({}),
            supports_audio: true,
            supports_text: false,
            supports_vision: false,
        });

        assert!(!supports_text(&config, "audio-only").expect("model should resolve"));
    }
}
