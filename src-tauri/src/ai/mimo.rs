use reqwest::Client;
use super::transport;

use super::types::*;

pub async fn transcribe(
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
        stream: None,
        max_tokens: None,
        stream_options: None,
        thinking: Some(ThinkingParam { thinking_type: "disabled".to_string() }),
        reasoning_effort: None,
        reasoning: None,
        provider: None,
        chat_template_kwargs: None,
    };

    let chat_resp =
        transport::chat(client, &url, &request, &[("api-key", api_key.to_string())], "MiMo").await?;

    let text = chat_resp
        .choices
        .as_ref()
        .and_then(|choices| choices.first())
        .and_then(|choice| choice.message.as_ref())
        .and_then(|msg| msg.content.as_ref())
        .ok_or_else(|| "No text in MiMo transcribe response".to_string())?;

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
        thinking: Some(ThinkingParam { thinking_type: "disabled".to_string() }),
        reasoning_effort: None,
        reasoning: None,
        provider: None,
        chat_template_kwargs: None,
    };

    let chat_resp =
        transport::chat(client, &url, &request, &[("api-key", api_key.to_string())], "MiMo").await?;

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
        stream: None,
        max_tokens: None,
        stream_options: None,
        thinking: Some(ThinkingParam { thinking_type: "disabled".to_string() }),
        reasoning_effort: None,
        reasoning: None,
        provider: None,
        chat_template_kwargs: None,
    };

    let chat_resp =
        transport::chat(client, &url, &request, &[("api-key", api_key.to_string())], "MiMo").await?;

    let text = chat_resp
        .choices
        .as_ref()
        .and_then(|choices| choices.first())
        .and_then(|choice| choice.message.as_ref())
        .and_then(|msg| msg.content.as_ref())
        .ok_or_else(|| "No text in MiMo extract_text response".to_string())?;

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
        max_tokens: None,
        stream_options: None,
        thinking: Some(ThinkingParam { thinking_type: "disabled".to_string() }),
        reasoning_effort: None,
        reasoning: None,
        provider: None,
        chat_template_kwargs: None,
    };

    transport::post(client, &url, &request, &[("api-key", api_key.to_string())], "MiMo").await?;

    Ok(())
}
