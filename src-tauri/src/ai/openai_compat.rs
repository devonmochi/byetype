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
        messages: vec![ChatMessage {
            role: "user".to_string(),
            content: ChatContent::Parts(vec![
                ChatContentPart::Text {
                    text: system_prompt.to_string(),
                },
                ChatContentPart::InputAudio {
                    input_audio: AudioData {
                        data: format!("data:;base64,{}", audio_base64),
                        format: "flac".to_string(),
                    },
                },
            ]),
        }],
        modalities: Some(vec!["text".to_string()]),
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
