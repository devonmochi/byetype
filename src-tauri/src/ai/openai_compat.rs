use reqwest::Client;
use super::transport;

use super::types::*;
use crate::config::types::{AudioInputMode, ThinkingConfig};

fn is_openrouter(base_url: &str) -> bool {
    base_url.contains("openrouter.ai")
}

/// OpenRouter 上的 Google 模型锁定 google-vertex/global 服务商。
/// OpenRouter 默认按价格加权分流,容易路由到 flex 省钱档(首字中位 12 秒以上);
/// 锁定标准档后实测转写延迟减半,且 30 分钟/24 小时可用率均为 100%。
/// allow_fallbacks=false: 失败不兜底,依赖上层 retry.rs 的重试逻辑。
fn openrouter_provider(base_url: &str, model: &str) -> Option<OpenRouterProvider> {
    if is_openrouter(base_url) && model.starts_with("google/") {
        Some(OpenRouterProvider {
            order: vec!["google-vertex/global".to_string()],
            allow_fallbacks: false,
        })
    } else {
        None
    }
}

/// 空对象 {} 视为未配置,不发送该字段。
/// OpenAI 官方等严格服务会拒绝未知参数,空对象原样发送会导致所有请求报错。
fn effective_kwargs(kwargs: Option<&serde_json::Value>) -> Option<serde_json::Value> {
    match kwargs {
        Some(v) if v.is_object() && !v.as_object().unwrap().is_empty() => Some(v.clone()),
        _ => None,
    }
}

/// OpenRouter 没有真正的 "关闭思考" 开关; thinking.enabled=false 时映射到 effort=minimal,
/// 这是 Gemini 3 系列最低的思考档位,最接近 "尽量不思考" 的语义。
/// Gemini 3.7 系列(如 google/gemini-3.7-flash)不支持 minimal,降级为 low。
fn openrouter_reasoning(thinking: Option<&ThinkingConfig>, model: &str) -> Option<OpenRouterReasoning> {
    // OpenRouter 的 effort 取值要小写: minimal / low / medium / high。
    // ByeType 的 ThinkingConfig.level 在前端以大写存储 (MINIMAL/LOW/MEDIUM/HIGH),需要转小写。
    let mut effort = match thinking {
        Some(cfg) if cfg.enabled => {
            let lvl = cfg.level.trim().to_lowercase();
            if lvl.is_empty() { "medium".to_string() } else { lvl }
        }
        _ => "minimal".to_string(),
    };
    if effort == "minimal" && model.contains("gemini-3.7") {
        effort = "low".to_string();
    }
    Some(OpenRouterReasoning { effort })
}

/// OpenAI 兼容请求头：Bearer 鉴权，走 OpenRouter 时再带上来源标识。
fn api_headers(api_key: &str, base_url: &str) -> Vec<(&'static str, String)> {
    let mut headers = vec![("Authorization", format!("Bearer {}", api_key))];
    if is_openrouter(base_url) {
        headers.push(("HTTP-Referer", "https://github.com/devonmochi/byetype".to_string()));
        headers.push(("X-Title", "ByeType".to_string()));
    }
    headers
}

pub async fn transcribe(
    client: &Client,
    audio_base64: &str,
    system_prompt: &str,
    api_key: &str,
    model: &str,
    base_url: &str,
    audio_input_mode: AudioInputMode,
    chat_template_kwargs: Option<&serde_json::Value>,
    thinking: Option<&ThinkingConfig>,
) -> Result<(String, TokenUsage), String> {
    let url = format!("{}/chat/completions", base_url.trim_end_matches('/'));

    let audio_part = match audio_input_mode {
        AudioInputMode::AudioUrl => ChatContentPart::AudioUrl {
            audio_url: AudioUrlData {
                url: format!("data:audio/flac;base64,{}", audio_base64),
            },
        },
        AudioInputMode::InputAudio => {
            let data = if is_openrouter(base_url) {
                audio_base64.to_string()
            } else {
                format!("data:;base64,{}", audio_base64)
            };
            ChatContentPart::InputAudio {
                input_audio: AudioData {
                    audio_type: None,
                    data,
                    format: "flac".to_string(),
                    sample_rate: None,
                },
            }
        }
    };

    let request = ChatCompletionRequest {
        model: model.to_string(),
        messages: vec![
            ChatMessage {
                role: "system".to_string(),
                content: ChatContent::Text(system_prompt.to_string()),
            },
            ChatMessage {
                role: "user".to_string(),
                content: ChatContent::Parts(vec![audio_part]),
            },
        ],
        modalities: Some(vec!["text".to_string()]),
        output_modalities: None,
        stream: None,
        max_tokens: None,
        stream_options: None,
        thinking: None,
        reasoning_effort: None,
        reasoning: if is_openrouter(base_url) { openrouter_reasoning(thinking, model) } else { None },
        provider: openrouter_provider(base_url, model),
        chat_template_kwargs: effective_kwargs(chat_template_kwargs),
    };

    let headers = api_headers(api_key, base_url);
    let chat_resp = transport::chat(client, &url, &request, &headers, "OpenAI-compat").await?;

    let text = chat_resp
        .choices
        .as_ref()
        .and_then(|choices| choices.first())
        .and_then(|choice| choice.message.as_ref())
        .and_then(|msg| msg.content.as_ref())
        .ok_or_else(|| "No text in OpenAI-compat response".to_string())?;

    let usage = chat_resp
        .usage
        .as_ref()
        .map(TokenUsage::from_chat)
        .unwrap_or_default();
    Ok((text.trim().to_string(), usage))
}

pub async fn optimize(
    client: &Client,
    text: &str,
    system_prompt: &str,
    api_key: &str,
    model: &str,
    base_url: &str,
    chat_template_kwargs: Option<&serde_json::Value>,
    thinking: Option<&ThinkingConfig>,
) -> Result<(String, TokenUsage), String> {
    let url = format!("{}/chat/completions", base_url.trim_end_matches('/'));

    let user_content = format!("<voice-input>\n{}\n</voice-input>", text);

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
        thinking: None,
        reasoning_effort: None,
        reasoning: if is_openrouter(base_url) { openrouter_reasoning(thinking, model) } else { None },
        provider: openrouter_provider(base_url, model),
        chat_template_kwargs: effective_kwargs(chat_template_kwargs),
    };

    let headers = api_headers(api_key, base_url);
    let chat_resp = transport::chat(client, &url, &request, &headers, "OpenAI-compat").await?;

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

pub async fn extract_text(
    client: &Client,
    image_base64: &str,
    system_prompt: &str,
    api_key: &str,
    model: &str,
    base_url: &str,
    chat_template_kwargs: Option<&serde_json::Value>,
    thinking: Option<&ThinkingConfig>,
) -> Result<(String, TokenUsage), String> {
    let url = format!("{}/chat/completions", base_url.trim_end_matches('/'));

    let request = ChatCompletionRequest {
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
        modalities: Some(vec!["text".to_string()]),
        output_modalities: None,
        stream: Some(false),
        max_tokens: None,
        stream_options: None,
        thinking: None,
        reasoning_effort: None,
        reasoning: if is_openrouter(base_url) { openrouter_reasoning(thinking, model) } else { None },
        provider: openrouter_provider(base_url, model),
        chat_template_kwargs: effective_kwargs(chat_template_kwargs),
    };

    let headers = api_headers(api_key, base_url);
    let chat_resp = transport::chat(client, &url, &request, &headers, "OpenAI-compat").await?;

    let text = chat_resp
        .choices
        .as_ref()
        .and_then(|choices| choices.first())
        .and_then(|choice| choice.message.as_ref())
        .and_then(|msg| msg.content.as_ref())
        .ok_or_else(|| "No text in OpenAI-compat extract_text response".to_string())?;

    let usage = chat_resp
        .usage
        .as_ref()
        .map(TokenUsage::from_chat)
        .unwrap_or_default();
    Ok((text.trim().to_string(), usage))
}

pub async fn qwen_omni_extract_text(
    client: &Client,
    image_base64: &str,
    system_prompt: &str,
    api_key: &str,
    model: &str,
    base_url: &str,
) -> Result<(String, TokenUsage), String> {
    let url = format!("{}/chat/completions", base_url.trim_end_matches('/'));

    let request = ChatCompletionRequest {
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
        modalities: Some(vec!["text".to_string()]),
        output_modalities: None,
        stream: Some(true),
        max_tokens: None,
        stream_options: Some(super::types::StreamOptions { include_usage: true }),
        thinking: None,
        reasoning_effort: None,
        reasoning: None,
        provider: openrouter_provider(base_url, model),
        chat_template_kwargs: None,
    };

    let body =
        transport::post(client, &url, &request, &transport::bearer(api_key), "Qwen Omni").await?;
    let (text, usage) = parse_sse(&body)?;
    if text.is_empty() {
        return Err("No text in Qwen Omni extract_text response".to_string());
    }
    Ok((text.trim().to_string(), usage))
}

pub async fn test_connectivity(
    client: &Client,
    api_key: &str,
    model: &str,
    base_url: &str,
    chat_template_kwargs: Option<&serde_json::Value>,
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
        max_tokens: None,
        stream_options: None,
        thinking: None,
        reasoning_effort: None,
        reasoning: None,
        provider: openrouter_provider(base_url, model),
        chat_template_kwargs: effective_kwargs(chat_template_kwargs),
    };

    transport::post(client, &url, &request, &transport::bearer(api_key), "OpenAI-compat").await?;

    Ok(())
}

/// Parse a complete SSE response body into a single text string plus token usage.
/// Iterates over `data: {...}` lines, extracts delta.content from each chunk, and concatenates.
/// usage 由 include_usage 生成的最后一个 chunk 携带，取最后一次出现的值。
fn parse_sse(body: &str) -> Result<(String, TokenUsage), String> {
    let mut result = String::new();
    let mut usage = TokenUsage::default();
    for line in body.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with(':') {
            continue;
        }
        if let Some(data) = line.strip_prefix("data: ") {
            let data = data.trim();
            if data == "[DONE]" {
                break;
            }
            let chunk: super::types::StreamChunk =
                serde_json::from_str(data).map_err(|e| format!("Failed to parse SSE chunk: {}", e))?;
            if let Some(chunk_usage) = chunk.usage.as_ref() {
                usage = TokenUsage::from_chat(chunk_usage);
            }
            if let Some(content) = chunk
                .choices
                .as_ref()
                .and_then(|c| c.first())
                .and_then(|c| c.delta.as_ref())
                .and_then(|d| d.content.as_ref())
            {
                result.push_str(content);
            }
        }
    }
    Ok((result, usage))
}

pub async fn qwen_omni_transcribe(
    client: &Client,
    audio_base64: &str,
    system_prompt: &str,
    api_key: &str,
    model: &str,
    base_url: &str,
) -> Result<(String, TokenUsage), String> {
    let url = format!("{}/chat/completions", base_url.trim_end_matches('/'));

    let request = ChatCompletionRequest {
        model: model.to_string(),
        messages: vec![
            ChatMessage {
                role: "system".to_string(),
                content: ChatContent::Text(system_prompt.to_string()),
            },
            ChatMessage {
                role: "user".to_string(),
                content: ChatContent::Parts(vec![ChatContentPart::InputAudio {
                    input_audio: AudioData {
                        audio_type: None,
                        data: format!("data:;base64,{}", audio_base64),
                        format: "flac".to_string(),
                        sample_rate: None,
                    },
                }]),
            },
        ],
        modalities: Some(vec!["text".to_string()]),
        output_modalities: None,
        stream: Some(true),
        max_tokens: None,
        stream_options: Some(super::types::StreamOptions { include_usage: true }),
        thinking: None,
        reasoning_effort: None,
        reasoning: None,
        provider: openrouter_provider(base_url, model),
        chat_template_kwargs: None,
    };

    let body =
        transport::post(client, &url, &request, &transport::bearer(api_key), "Qwen Omni").await?;
    let (text, usage) = parse_sse(&body)?;
    if text.is_empty() {
        return Err("No text in Qwen Omni transcribe response".to_string());
    }
    Ok((text.trim().to_string(), usage))
}

pub async fn qwen_omni_optimize(
    client: &Client,
    text: &str,
    system_prompt: &str,
    api_key: &str,
    model: &str,
    base_url: &str,
) -> Result<(String, TokenUsage), String> {
    let url = format!("{}/chat/completions", base_url.trim_end_matches('/'));

    let user_content = format!("<voice-input>\n{}\n</voice-input>", text);

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
        modalities: Some(vec!["text".to_string()]),
        output_modalities: None,
        stream: Some(true),
        max_tokens: None,
        stream_options: Some(super::types::StreamOptions { include_usage: true }),
        thinking: None,
        reasoning_effort: None,
        reasoning: None,
        provider: openrouter_provider(base_url, model),
        chat_template_kwargs: None,
    };

    let body =
        transport::post(client, &url, &request, &transport::bearer(api_key), "Qwen Omni").await?;
    let (result, usage) = parse_sse(&body)?;
    if result.is_empty() {
        return Ok((text.to_string(), usage));
    }
    Ok((result.trim().to_string(), usage))
}

pub async fn qwen_omni_test_connectivity(
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
        modalities: Some(vec!["text".to_string()]),
        output_modalities: None,
        stream: Some(true),
        max_tokens: Some(32),
        stream_options: Some(super::types::StreamOptions { include_usage: true }),
        thinking: None,
        reasoning_effort: None,
        reasoning: None,
        provider: openrouter_provider(base_url, model),
        chat_template_kwargs: None,
    };

    transport::post(client, &url, &request, &transport::bearer(api_key), "Qwen Omni").await?;

    Ok(())
}

#[cfg(test)]
mod kwargs_tests {
    use super::effective_kwargs;
    use serde_json::json;

    #[test]
    fn empty_object_kwargs_is_not_sent() {
        assert!(effective_kwargs(Some(&json!({}))).is_none());
        assert!(effective_kwargs(None).is_none());
    }

    #[test]
    fn non_empty_object_kwargs_is_sent() {
        let kwargs = json!({"enable_thinking": false});
        assert_eq!(effective_kwargs(Some(&kwargs)), Some(kwargs));
    }
}

#[cfg(test)]
mod sse_tests {
    use super::parse_sse;

    #[test]
    fn extracts_text_and_usage_from_stream() {
        let body = concat!(
            "data: {\"choices\":[{\"delta\":{\"content\":\"你\"}}]}\n\n",
            "data: {\"choices\":[{\"delta\":{\"content\":\"好\"}}]}\n\n",
            "data: {\"choices\":[],\"usage\":{\"prompt_tokens\":25,\"completion_tokens\":8,\"total_tokens\":33}}\n\n",
            "data: [DONE]\n\n",
        );
        let (text, usage) = parse_sse(body).unwrap();
        assert_eq!(text, "你好");
        assert_eq!(usage.prompt_tokens, 25);
        assert_eq!(usage.completion_tokens, 8);
    }

    #[test]
    fn usage_defaults_to_zero_when_missing() {
        let body = "data: {\"choices\":[{\"delta\":{\"content\":\"hi\"}}]}\n\ndata: [DONE]\n\n";
        let (text, usage) = parse_sse(body).unwrap();
        assert_eq!(text, "hi");
        assert_eq!((usage.prompt_tokens, usage.completion_tokens), (0, 0));
    }
}
