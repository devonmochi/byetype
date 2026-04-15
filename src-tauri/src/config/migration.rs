use serde_json::Value;

pub fn migrate_if_needed(raw: &mut Value) -> bool {
    let mut migrated = false;

    // 迁移1：旧 model 字段 → modelId
    let needs_model_migration = raw
        .get("transcribe")
        .and_then(|t| t.get("model"))
        .and_then(|m| m.as_str())
        .is_some()
        && raw
            .get("transcribe")
            .and_then(|t| t.get("modelId"))
            .is_none();

    if needs_model_migration {
        let gemini_api_key = raw.get("transcribe").and_then(|t| t.get("geminiApiKey")).and_then(|v| v.as_str()).unwrap_or_default().to_string();
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
                    "supportsVision": false,
                }));
                "migrated-openai-compat".to_string()
            } else {
                String::new()
            }
        };

        raw["models"] = serde_json::json!({
            "builtinApiKeys": { "gemini": gemini_api_key, "deepseek": "", "dashscope": "", "openrouter": "" },
            "custom": custom_models,
        });

        let thinking = raw.get("transcribe").and_then(|t| t.get("thinking")).cloned().unwrap_or(serde_json::json!({ "enabled": false, "budget": 1024, "level": "LOW" }));
        let prompts = raw.get("transcribe").and_then(|t| t.get("prompts")).cloned().unwrap_or(serde_json::json!({ "agent": "", "rules": "", "vocabulary": "" }));
        raw["transcribe"] = serde_json::json!({ "modelId": transcribe_model_id, "thinking": thinking, "prompts": prompts });

        let opt_enabled = raw.get("optimize").and_then(|o| o.get("enabled")).and_then(|v| v.as_bool()).unwrap_or(false);
        let opt_thinking = raw.get("optimize").and_then(|o| o.get("thinking")).cloned().unwrap_or(serde_json::json!({ "enabled": false, "budget": 1024, "level": "LOW" }));
        let opt_prompt = raw.get("optimize").and_then(|o| o.get("prompt")).and_then(|v| v.as_str()).unwrap_or_default().to_string();
        raw["optimize"] = serde_json::json!({ "enabled": opt_enabled, "modelId": optimize_model_id, "thinking": opt_thinking, "prompt": opt_prompt });

        migrated = true;
    }

    // 迁移2：optimize → voiceTemplates
    if raw.get("optimize").is_some() && raw.get("voiceTemplates").is_none() {
        migrate_optimize_to_voice_templates(raw);
        migrated = true;
    }

    migrated
}

fn migrate_optimize_to_voice_templates(raw: &mut Value) {
    let opt = match raw.get("optimize") {
        Some(v) => v.clone(),
        None => return,
    };

    let model_id = opt.get("modelId").and_then(|v| v.as_str()).unwrap_or_default().to_string();
    let thinking = opt.get("thinking").cloned().unwrap_or(
        serde_json::json!({ "enabled": false, "budget": 1024, "level": "LOW" })
    );
    let custom_prompt = opt.get("prompt").and_then(|v| v.as_str()).unwrap_or_default().to_string();
    let enabled = opt.get("enabled").and_then(|v| v.as_bool()).unwrap_or(false);

    // Build voice templates list
    let templates = serde_json::json!([
        { "id": "voice-optimize", "name": "文本优化", "prompt": custom_prompt },
        { "id": "voice-translate", "name": "翻译", "prompt": "" },
        { "id": "voice-custom", "name": "自定义", "prompt": "" },
    ]);

    raw["voiceTemplates"] = serde_json::json!({
        "modelId": model_id,
        "thinking": thinking,
        "templates": templates,
    });

    // Set shortcut template bindings
    if let Some(general) = raw.get_mut("general") {
        if enabled {
            general["shortcutTemplate"] = serde_json::json!("voice-optimize");
        } else {
            general["shortcutTemplate"] = serde_json::json!("");
        }
        general["shortcut2"] = serde_json::json!("F5");
        general["shortcut2Template"] = serde_json::json!("voice-translate");
        general["extractShortcut2"] = serde_json::json!("F7");
        general["extractShortcutTemplate"] = serde_json::json!("image-extract");
        general["extractShortcut2Template"] = serde_json::json!("image-translate");
    }

    // Add default image templates to extract config
    if let Some(extract) = raw.get_mut("extract") {
        if extract.get("templates").is_none() {
            extract["templates"] = serde_json::json!([
                { "id": "image-extract", "name": "文字识别", "prompt": "" },
                { "id": "image-translate", "name": "翻译", "prompt": "" },
                { "id": "image-custom", "name": "自定义", "prompt": "" },
            ]);
        }
    }

    // Remove old optimize key
    if let Some(obj) = raw.as_object_mut() {
        obj.remove("optimize");
    }
}

fn model_name_to_builtin_id(model_name: &str) -> String {
    match model_name {
        "gemini-3-flash-preview" => "builtin-gemini-3-flash".to_string(),
        "gemini-3.1-flash-lite-preview" => "builtin-gemini-3.1-flash-lite".to_string(),
        _ => "builtin-gemini-3-flash".to_string(),
    }
}
