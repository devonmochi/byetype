use reqwest::Client;

use crate::config::types::ThinkingConfig;
use super::types::*;

/// 根据 ThinkingConfig 与可选的 reasoning_effort 配置构造 DeepSeek 的请求参数。
/// - enabled=false → thinking:disabled,不发 reasoning_effort
/// - enabled=true  → thinking:enabled,reasoning_effort 直接使用配置值(默认 "high")。
///   官方仅支持 "high" / "max" 两档,非法值由调用方保证。
fn build_thinking_params(
    thinking: &ThinkingConfig,
    reasoning_effort: Option<&str>,
) -> (Option<ThinkingParam>, Option<String>) {
    if !thinking.enabled {
        return (
            Some(ThinkingParam { thinking_type: "disabled".to_string() }),
            None,
        );
    }
    let effort = reasoning_effort.unwrap_or("high").to_string();
    (
        Some(ThinkingParam { thinking_type: "enabled".to_string() }),
        Some(effort),
    )
}

pub async fn optimize(
    client: &Client,
    text: &str,
    system_prompt: &str,
    api_key: &str,
    model: &str,
    base_url: &str,
    thinking: &ThinkingConfig,
    reasoning_effort: Option<&str>,
) -> Result<String, String> {
    let url = format!("{}/chat/completions", base_url.trim_end_matches('/'));
    let user_content = format!("<voice-input>\n{}\n</voice-input>", text);
    let (thinking_param, reasoning_effort) = build_thinking_params(thinking, reasoning_effort);

    let request = ChatCompletionRequest {
        model: model.to_string(),
        messages: vec![
            ChatMessage {
                role: "system".to_string(),
                content: ChatContent::Text(system_prompt.to_string()),
            },
            ChatMessage {
                role: "user".to_string(),
                content: ChatContent::Text(user_content),
            },
        ],
        modalities: None,
        output_modalities: None,
        stream: None,
        max_tokens: None,
        stream_options: None,
        thinking: thinking_param,
        reasoning_effort,
    };

    let resp = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&request)
        .send()
        .await
        .map_err(|e| format!("DeepSeek optimize request failed: {}", e))?;

    let status = resp.status();
    let body = resp
        .text()
        .await
        .map_err(|e| format!("Failed to read DeepSeek response: {}", e))?;

    if !status.is_success() {
        return Err(format!("DeepSeek API error ({}): {}", status, body));
    }

    let chat_resp: ChatCompletionResponse = serde_json::from_str(&body)
        .map_err(|e| format!("Failed to parse DeepSeek response: {}", e))?;

    let result = chat_resp
        .choices
        .as_ref()
        .and_then(|choices| choices.first())
        .and_then(|choice| choice.message.as_ref())
        .and_then(|msg| msg.content.as_ref())
        .map(|s| s.trim().to_string())
        .unwrap_or_default();

    if result.is_empty() {
        return Ok(text.to_string());
    }
    Ok(result)
}

pub async fn transcribe(
    _client: &Client,
    _audio_base64: &str,
    _system_prompt: &str,
    _api_key: &str,
    _model: &str,
    _base_url: &str,
) -> Result<String, String> {
    Err("DeepSeek 模型不支持音频转写,请选择其他模型".to_string())
}

pub async fn extract_text(
    _client: &Client,
    _image_base64: &str,
    _system_prompt: &str,
    _api_key: &str,
    _model: &str,
    _base_url: &str,
) -> Result<String, String> {
    Err("DeepSeek 模型不支持图像识别,请选择其他模型".to_string())
}

pub async fn test_connectivity(
    client: &Client,
    api_key: &str,
    model: &str,
    base_url: &str,
) -> Result<(), String> {
    let url = format!("{}/chat/completions", base_url.trim_end_matches('/'));

    let request = ChatCompletionRequest {
        model: model.to_string(),
        messages: vec![ChatMessage {
            role: "user".to_string(),
            content: ChatContent::Text("hi".to_string()),
        }],
        modalities: None,
        output_modalities: None,
        stream: None,
        max_tokens: Some(8),
        stream_options: None,
        thinking: Some(ThinkingParam { thinking_type: "disabled".to_string() }),
        reasoning_effort: None,
    };

    let resp = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&request)
        .send()
        .await
        .map_err(|e| format!("DeepSeek connectivity test failed: {}", e))?;

    let status = resp.status();
    if !status.is_success() {
        let body = resp
            .text()
            .await
            .map_err(|e| format!("Failed to read DeepSeek response: {}", e))?;
        return Err(format!("DeepSeek API error ({}): {}", status, body));
    }
    Ok(())
}
