use reqwest::Client;

use super::types::*;
use crate::config::types::ThinkingConfig;

pub fn build_thinking_config(
    _model: &str,
    thinking: &ThinkingConfig,
) -> Option<GeminiGenerationConfig> {
    if !thinking.enabled {
        return None;
    }
    Some(GeminiGenerationConfig {
        thinking: Some(GeminiThinkingConfig::Level {
            thinking_level: thinking.level.clone(),
        }),
    })
}

pub async fn transcribe(
    client: &Client,
    audio_base64: &str,
    system_prompt: &str,
    api_key: &str,
    model: &str,
    thinking: &ThinkingConfig,
) -> Result<String, String> {
    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
        model, api_key
    );

    let system_instruction = if system_prompt.is_empty() {
        None
    } else {
        Some(GeminiContent {
            role: None,
            parts: vec![GeminiPart::Text {
                text: system_prompt.to_string(),
            }],
        })
    };

    let request = GeminiRequest {
        system_instruction,
        contents: vec![GeminiContent {
            role: Some("user".to_string()),
            parts: vec![GeminiPart::InlineData {
                inline_data: GeminiInlineData {
                    mime_type: "audio/wav".to_string(),
                    data: audio_base64.to_string(),
                },
            }],
        }],
        generation_config: build_thinking_config(model, thinking),
    };

    let resp = client
        .post(&url)
        .json(&request)
        .send()
        .await
        .map_err(|e| format!("Gemini transcribe request failed: {}", e))?;

    let status = resp.status();
    let body = resp
        .text()
        .await
        .map_err(|e| format!("Failed to read Gemini response: {}", e))?;

    if !status.is_success() {
        return Err(format!("Gemini API error ({}): {}", status, body));
    }

    let gemini_resp: GeminiResponse =
        serde_json::from_str(&body).map_err(|e| format!("Failed to parse Gemini response: {}", e))?;

    extract_gemini_text(&gemini_resp)
}

pub async fn optimize(
    client: &Client,
    text: &str,
    system_prompt: &str,
    api_key: &str,
    model: &str,
    thinking: &ThinkingConfig,
) -> Result<String, String> {
    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
        model, api_key
    );

    let user_content = format!("<voice-input>\n{}\n</voice-input>\n\n{}", text, system_prompt);

    let request = GeminiRequest {
        system_instruction: None,
        contents: vec![GeminiContent {
            role: Some("user".to_string()),
            parts: vec![GeminiPart::Text {
                text: user_content,
            }],
        }],
        generation_config: build_thinking_config(model, thinking),
    };

    let resp = client
        .post(&url)
        .json(&request)
        .send()
        .await
        .map_err(|e| format!("Gemini optimize request failed: {}", e))?;

    let status = resp.status();
    let body = resp
        .text()
        .await
        .map_err(|e| format!("Failed to read Gemini response: {}", e))?;

    if !status.is_success() {
        return Err(format!("Gemini API error ({}): {}", status, body));
    }

    let gemini_resp: GeminiResponse =
        serde_json::from_str(&body).map_err(|e| format!("Failed to parse Gemini response: {}", e))?;

    extract_gemini_text(&gemini_resp)
}

fn extract_gemini_text(resp: &GeminiResponse) -> Result<String, String> {
    let candidates = resp
        .candidates
        .as_ref()
        .ok_or_else(|| "No candidates in Gemini response".to_string())?;

    let candidate = candidates
        .first()
        .ok_or_else(|| "Empty candidates in Gemini response".to_string())?;

    let content = candidate
        .content
        .as_ref()
        .ok_or_else(|| "No content in Gemini candidate".to_string())?;

    let parts = content
        .parts
        .as_ref()
        .ok_or_else(|| "No parts in Gemini content".to_string())?;

    // Get the last text part (skipping thinking parts)
    let text = parts
        .iter()
        .rev()
        .find_map(|p| p.text.as_ref())
        .ok_or_else(|| "No text found in Gemini response parts".to_string())?;

    Ok(text.trim().to_string())
}
