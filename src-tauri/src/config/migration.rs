use serde_json::Value;

pub fn migrate_if_needed(raw: &mut Value) -> bool {
    let needs_migration = raw
        .get("transcribe")
        .and_then(|t| t.get("model"))
        .and_then(|m| m.as_str())
        .is_some()
        && raw
            .get("transcribe")
            .and_then(|t| t.get("modelId"))
            .is_none();

    if !needs_migration {
        return false;
    }

    let gemini_api_key = raw.get("transcribe").and_then(|t| t.get("geminiApiKey")).and_then(|v| v.as_str()).unwrap_or_default().to_string();
    let qwen_api_key = raw.get("transcribe").and_then(|t| t.get("qwenApiKey")).and_then(|v| v.as_str()).unwrap_or_default().to_string();
    let old_model = raw.get("transcribe").and_then(|t| t.get("model")).and_then(|v| v.as_str()).unwrap_or("gemini-3-flash-preview").to_string();
    let transcribe_model_id = model_name_to_builtin_id(&old_model);

    let mut custom_models: Vec<Value> = Vec::new();
    let optimize_type = raw.get("optimize").and_then(|o| o.get("type")).and_then(|v| v.as_str()).unwrap_or("openai-compat").to_string();

    let optimize_model_id = if optimize_type == "gemini" {
        let gemini_model = raw.get("optimize").and_then(|o| o.get("geminiModel")).and_then(|v| v.as_str()).unwrap_or("gemini-3-flash-preview").to_string();
        model_name_to_builtin_id(&gemini_model)
    } else {
        let compat = raw.get("optimize").and_then(|o| o.get("openaiCompat"));
        let provider_name = compat.and_then(|c| c.get("providerName")).and_then(|v| v.as_str()).unwrap_or_default().to_string();
        let base_url = compat.and_then(|c| c.get("baseUrl")).and_then(|v| v.as_str()).unwrap_or_default().to_string();
        let model = compat.and_then(|c| c.get("model")).and_then(|v| v.as_str()).unwrap_or_default().to_string();
        let api_key = compat.and_then(|c| c.get("apiKey")).and_then(|v| v.as_str()).unwrap_or_default().to_string();

        if !base_url.is_empty() || !model.is_empty() {
            let provider = if provider_name.is_empty() { "OpenAI 兼容".to_string() } else { provider_name };
            custom_models.push(serde_json::json!({
                "id": "migrated-openai-compat",
                "provider": provider,
                "model": model,
                "protocol": "openai-compat",
                "baseUrl": base_url,
                "apiKey": api_key,
                "supportsAudio": false,
                "supportsText": true,
            }));
            "migrated-openai-compat".to_string()
        } else {
            String::new()
        }
    };

    raw["models"] = serde_json::json!({
        "builtinApiKeys": { "gemini": gemini_api_key, "qwen": qwen_api_key },
        "custom": custom_models,
    });

    let thinking = raw.get("transcribe").and_then(|t| t.get("thinking")).cloned().unwrap_or(serde_json::json!({ "enabled": false, "budget": 1024, "level": "LOW" }));
    let prompts = raw.get("transcribe").and_then(|t| t.get("prompts")).cloned().unwrap_or(serde_json::json!({ "agent": "", "rules": "", "vocabulary": "" }));
    raw["transcribe"] = serde_json::json!({ "modelId": transcribe_model_id, "thinking": thinking, "prompts": prompts });

    let opt_enabled = raw.get("optimize").and_then(|o| o.get("enabled")).and_then(|v| v.as_bool()).unwrap_or(false);
    let opt_thinking = raw.get("optimize").and_then(|o| o.get("thinking")).cloned().unwrap_or(serde_json::json!({ "enabled": false, "budget": 1024, "level": "LOW" }));
    let opt_prompt = raw.get("optimize").and_then(|o| o.get("prompt")).and_then(|v| v.as_str()).unwrap_or_default().to_string();
    raw["optimize"] = serde_json::json!({ "enabled": opt_enabled, "modelId": optimize_model_id, "thinking": opt_thinking, "prompt": opt_prompt });

    true
}

fn model_name_to_builtin_id(model_name: &str) -> String {
    match model_name {
        "gemini-3-flash-preview" => "builtin-gemini-3-flash".to_string(),
        "gemini-3.1-flash-lite-preview" => "builtin-gemini-3.1-flash-lite".to_string(),
        "qwen3-omni-flash" => "builtin-qwen3-omni-flash".to_string(),
        _ => "builtin-gemini-3-flash".to_string(),
    }
}
