use reqwest::Client;

use super::types::*;

pub async fn transcribe(
    client: &Client,
    audio_base64: &str,
    system_prompt: &str,
    api_key: &str,
    model: &str,
    base_url: &str,
) -> Result<String, String> {
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
    };

    let resp = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&request)
        .send()
        .await
        .map_err(|e| format!("OpenAI-compat transcribe request failed: {}", e))?;

    let status = resp.status();
    let body = resp
        .text()
        .await
        .map_err(|e| format!("Failed to read OpenAI-compat response: {}", e))?;

    if !status.is_success() {
        return Err(format!("OpenAI-compat API error ({}): {}", status, body));
    }

    let chat_resp: ChatCompletionResponse =
        serde_json::from_str(&body).map_err(|e| format!("Failed to parse OpenAI-compat response: {}", e))?;

    let text = chat_resp
        .choices
        .as_ref()
        .and_then(|choices| choices.first())
        .and_then(|choice| choice.message.as_ref())
        .and_then(|msg| msg.content.as_ref())
        .ok_or_else(|| "No text in OpenAI-compat response".to_string())?;

    Ok(text.trim().to_string())
}

pub async fn optimize(
    client: &Client,
    text: &str,
    system_prompt: &str,
    api_key: &str,
    model: &str,
    base_url: &str,
) -> Result<String, String> {
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
    };

    let resp = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&request)
        .send()
        .await
        .map_err(|e| format!("OpenAI-compat optimize request failed: {}", e))?;

    let status = resp.status();
    let body = resp
        .text()
        .await
        .map_err(|e| format!("Failed to read OpenAI-compat response: {}", e))?;

    if !status.is_success() {
        return Err(format!("OpenAI-compat API error ({}): {}", status, body));
    }

    let chat_resp: ChatCompletionResponse = serde_json::from_str(&body)
        .map_err(|e| format!("Failed to parse OpenAI-compat response: {}", e))?;

    let result = chat_resp
        .choices
        .as_ref()
        .and_then(|choices| choices.first())
        .and_then(|choice| choice.message.as_ref())
        .and_then(|msg| msg.content.as_ref())
        .map(|s| s.trim().to_string())
        .unwrap_or_default();

    // On empty response, return original text
    if result.is_empty() {
        return Ok(text.to_string());
    }

    Ok(result)
}

pub async fn extract_text(
    client: &Client,
    image_base64: &str,
    system_prompt: &str,
    api_key: &str,
    model: &str,
    base_url: &str,
) -> Result<String, String> {
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
    };

    let resp = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&request)
        .send()
        .await
        .map_err(|e| format!("OpenAI-compat extract_text request failed: {}", e))?;

    let status = resp.status();
    let body = resp
        .text()
        .await
        .map_err(|e| format!("Failed to read OpenAI-compat response: {}", e))?;

    if !status.is_success() {
        return Err(format!("OpenAI-compat API error ({}): {}", status, body));
    }

    let chat_resp: ChatCompletionResponse =
        serde_json::from_str(&body).map_err(|e| format!("Failed to parse OpenAI-compat response: {}", e))?;

    let text = chat_resp
        .choices
        .as_ref()
        .and_then(|choices| choices.first())
        .and_then(|choice| choice.message.as_ref())
        .and_then(|msg| msg.content.as_ref())
        .ok_or_else(|| "No text in OpenAI-compat extract_text response".to_string())?;

    Ok(text.trim().to_string())
}

pub async fn qwen_omni_extract_text(
    client: &Client,
    image_base64: &str,
    system_prompt: &str,
    api_key: &str,
    model: &str,
    base_url: &str,
) -> Result<String, String> {
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
    };

    let resp = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&request)
        .send()
        .await
        .map_err(|e| format!("Qwen Omni extract_text request failed: {}", e))?;

    let status = resp.status();
    let body = resp
        .text()
        .await
        .map_err(|e| format!("Failed to read Qwen Omni response: {}", e))?;

    if !status.is_success() {
        return Err(format!("Qwen Omni API error ({}): {}", status, body));
    }

    let text = parse_sse_text(&body)?;
    if text.is_empty() {
        return Err("No text in Qwen Omni extract_text response".to_string());
    }
    Ok(text.trim().to_string())
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
    };

    let resp = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&request)
        .send()
        .await
        .map_err(|e| format!("OpenAI-compat connectivity test failed: {}", e))?;

    let status = resp.status();
    if !status.is_success() {
        let body = resp
            .text()
            .await
            .map_err(|e| format!("Failed to read OpenAI-compat response: {}", e))?;
        return Err(format!("OpenAI-compat API error ({}): {}", status, body));
    }

    Ok(())
}

/// Parse a complete SSE response body into a single text string.
/// Iterates over `data: {...}` lines, extracts delta.content from each chunk, and concatenates.
fn parse_sse_text(body: &str) -> Result<String, String> {
    let mut result = String::new();
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
    Ok(result)
}

pub async fn qwen_omni_transcribe(
    client: &Client,
    audio_base64: &str,
    system_prompt: &str,
    api_key: &str,
    model: &str,
    base_url: &str,
) -> Result<String, String> {
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
    };

    let resp = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&request)
        .send()
        .await
        .map_err(|e| format!("Qwen Omni transcribe request failed: {}", e))?;

    let status = resp.status();
    let body = resp
        .text()
        .await
        .map_err(|e| format!("Failed to read Qwen Omni response: {}", e))?;

    if !status.is_success() {
        return Err(format!("Qwen Omni API error ({}): {}", status, body));
    }

    let text = parse_sse_text(&body)?;
    if text.is_empty() {
        return Err("No text in Qwen Omni transcribe response".to_string());
    }
    Ok(text.trim().to_string())
}

pub async fn qwen_omni_optimize(
    client: &Client,
    text: &str,
    system_prompt: &str,
    api_key: &str,
    model: &str,
    base_url: &str,
) -> Result<String, String> {
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
    };

    let resp = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&request)
        .send()
        .await
        .map_err(|e| format!("Qwen Omni optimize request failed: {}", e))?;

    let status = resp.status();
    let body = resp
        .text()
        .await
        .map_err(|e| format!("Failed to read Qwen Omni response: {}", e))?;

    if !status.is_success() {
        return Err(format!("Qwen Omni API error ({}): {}", status, body));
    }

    let result = parse_sse_text(&body)?;
    if result.is_empty() {
        return Ok(text.to_string());
    }
    Ok(result.trim().to_string())
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
    };

    let resp = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&request)
        .send()
        .await
        .map_err(|e| format!("Qwen Omni connectivity test failed: {}", e))?;

    let status = resp.status();
    if !status.is_success() {
        let body = resp
            .text()
            .await
            .map_err(|e| format!("Failed to read Qwen Omni response: {}", e))?;
        return Err(format!("Qwen Omni API error ({}): {}", status, body));
    }

    Ok(())
}
