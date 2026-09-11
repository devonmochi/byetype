use reqwest::Client;

use super::types::*;
use crate::config::types::ThinkingConfig;

/// 根据 ThinkingConfig 与可选的 reasoning_effort 配置构造 DeepSeek 的请求参数。
/// - enabled=false → thinking:disabled,不发 reasoning_effort
/// - enabled=true  → thinking:enabled,reasoning_effort 直接使用配置值(默认 "high")。
///   官方仅支持 "low" / "high" / "max" 三档,非法值由调用方保证。
fn build_thinking_params(
    thinking: &ThinkingConfig,
    reasoning_effort: Option<&str>,
) -> (Option<ThinkingParam>, Option<String>) {
    if !thinking.enabled {
        return (
            Some(ThinkingParam {
                thinking_type: "disabled".to_string(),
            }),
            None,
        );
    }
    let effort = reasoning_effort.unwrap_or("high").to_string();
    (
        Some(ThinkingParam {
            thinking_type: "enabled".to_string(),
        }),
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
) -> Result<(String, TokenUsage), String> {
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
        reasoning: None,
        provider: None,
        chat_template_kwargs: None,
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

    // 空响应时返回原文，但调用确实发生，用量照常上报
    let usage = chat_resp
        .usage
        .as_ref()
        .map(TokenUsage::from_chat)
        .unwrap_or_default();
    if result.is_empty() {
        return Ok((text.to_string(), usage));
    }
    Ok((result, usage))
}

pub async fn transcribe(
    _client: &Client,
    _audio_base64: &str,
    _system_prompt: &str,
    _api_key: &str,
    _model: &str,
    _base_url: &str,
) -> Result<(String, TokenUsage), String> {
    Err("DeepSeek 模型不支持音频转写,请选择其他模型".to_string())
}

/// 图像识别请求体。图片按官方要求只放在 user 消息的 content 数组里,用 image_url 块内联
/// data URI(base64 PNG)。不传 detail,让服务端按默认保留原图分辨率。
/// 思考强度沿用「文本优化模型」那一栏的 reasoning_effort 设置。
fn build_extract_request(
    model: &str,
    system_prompt: &str,
    image_base64: &str,
    thinking: &ThinkingConfig,
    reasoning_effort: Option<&str>,
) -> ChatCompletionRequest {
    let (thinking_param, reasoning_effort) = build_thinking_params(thinking, reasoning_effort);

    ChatCompletionRequest {
        model: model.to_string(),
        messages: vec![
            ChatMessage {
                role: "system".to_string(),
                content: ChatContent::Text(system_prompt.to_string()),
            },
            ChatMessage {
                role: "user".to_string(),
                content: ChatContent::Parts(vec![ChatContentPart::ImageUrl {
                    image_url: ImageUrlData {
                        url: format!("data:image/png;base64,{}", image_base64),
                    },
                }]),
            },
        ],
        modalities: None,
        output_modalities: None,
        stream: None,
        max_tokens: None,
        stream_options: None,
        thinking: thinking_param,
        reasoning_effort,
        reasoning: None,
        provider: None,
        chat_template_kwargs: None,
    }
}

pub async fn extract_text(
    client: &Client,
    image_base64: &str,
    system_prompt: &str,
    api_key: &str,
    model: &str,
    base_url: &str,
    thinking: &ThinkingConfig,
    reasoning_effort: Option<&str>,
) -> Result<(String, TokenUsage), String> {
    let url = format!("{}/chat/completions", base_url.trim_end_matches('/'));
    let request = build_extract_request(
        model,
        system_prompt,
        image_base64,
        thinking,
        reasoning_effort,
    );

    let resp = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&request)
        .send()
        .await
        .map_err(|e| format!("DeepSeek extract_text request failed: {}", e))?;

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

    let text = chat_resp
        .choices
        .as_ref()
        .and_then(|choices| choices.first())
        .and_then(|choice| choice.message.as_ref())
        .and_then(|msg| msg.content.as_ref())
        .filter(|content| !content.trim().is_empty())
        .ok_or_else(|| "No text in DeepSeek extract_text response".to_string())?;

    let usage = chat_resp
        .usage
        .as_ref()
        .map(TokenUsage::from_chat)
        .unwrap_or_default();
    Ok((text.trim().to_string(), usage))
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
        thinking: Some(ThinkingParam {
            thinking_type: "disabled".to_string(),
        }),
        reasoning_effort: None,
        reasoning: None,
        provider: None,
        chat_template_kwargs: None,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preserves_low_reasoning_effort_when_thinking_is_enabled() {
        let thinking = ThinkingConfig {
            enabled: true,
            budget: 1024,
            level: "LOW".to_string(),
        };

        let (thinking_param, reasoning_effort) = build_thinking_params(&thinking, Some("low"));

        assert_eq!(thinking_param.unwrap().thinking_type, "enabled");
        assert_eq!(reasoning_effort.as_deref(), Some("low"));
    }

    #[test]
    fn extract_request_sends_inline_png_in_user_message() {
        let thinking = ThinkingConfig {
            enabled: false,
            budget: 1024,
            level: "LOW".to_string(),
        };

        let request = build_extract_request(
            "deepseek-flash",
            "识别图片里的文字",
            "QUJD",
            &thinking,
            None,
        );
        let value = serde_json::to_value(&request).unwrap();

        assert_eq!(value["model"], "deepseek-flash");
        assert_eq!(value["messages"][0]["role"], "system");
        assert_eq!(value["messages"][0]["content"], "识别图片里的文字");
        assert_eq!(value["messages"][1]["role"], "user");
        assert_eq!(value["messages"][1]["content"][0]["type"], "image_url");
        assert_eq!(
            value["messages"][1]["content"][0]["image_url"]["url"],
            "data:image/png;base64,QUJD"
        );
    }

    #[test]
    fn extract_request_disables_thinking_by_default() {
        let thinking = ThinkingConfig::default();

        let request = build_extract_request("deepseek-flash", "", "QUJD", &thinking, None);
        let value = serde_json::to_value(&request).unwrap();

        // 官方默认开启思考模式,思考关闭时必须显式关掉,否则每次识别都白跑一段思维链
        assert_eq!(value["thinking"]["type"], "disabled");
        assert!(value.get("reasoning_effort").is_none());
    }

    #[test]
    fn extract_request_uses_configured_reasoning_effort() {
        let thinking = ThinkingConfig {
            enabled: true,
            budget: 1024,
            level: "LOW".to_string(),
        };

        let request = build_extract_request("deepseek-flash", "", "QUJD", &thinking, Some("low"));
        let value = serde_json::to_value(&request).unwrap();

        assert_eq!(value["thinking"]["type"], "enabled");
        assert_eq!(value["reasoning_effort"], "low");
    }
}
