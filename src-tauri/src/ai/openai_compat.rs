use reqwest::Client;

use super::types::*;
use crate::config::types::OpenAICompatConfig;

pub async fn optimize(
    client: &Client,
    text: &str,
    system_prompt: &str,
    compat_config: &OpenAICompatConfig,
) -> Result<String, String> {
    let url = format!("{}/chat/completions", compat_config.base_url.trim_end_matches('/'));

    let user_content = format!("<voice-input>\n{}\n</voice-input>\n\n{}", text, system_prompt);

    let request = ChatCompletionRequest {
        model: compat_config.model.clone(),
        messages: vec![ChatMessage {
            role: "user".to_string(),
            content: ChatContent::Text(user_content),
        }],
        modalities: None,
    };

    let resp = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", compat_config.api_key))
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
