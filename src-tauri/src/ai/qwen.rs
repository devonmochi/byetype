use reqwest::Client;

use super::types::*;

pub async fn transcribe(
    client: &Client,
    audio_base64: &str,
    system_prompt: &str,
    api_key: &str,
) -> Result<String, String> {
    let url = "https://dashscope.aliyuncs.com/compatible-mode/v1/chat/completions";

    let request = ChatCompletionRequest {
        model: "qwen3-omni-flash".to_string(),
        messages: vec![ChatMessage {
            role: "user".to_string(),
            content: ChatContent::Parts(vec![
                ChatContentPart::Text {
                    text: system_prompt.to_string(),
                },
                ChatContentPart::InputAudio {
                    input_audio: AudioData {
                        data: format!("data:;base64,{}", audio_base64),
                        format: "wav".to_string(),
                    },
                },
            ]),
        }],
        modalities: Some(vec!["text".to_string()]),
    };

    let resp = client
        .post(url)
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&request)
        .send()
        .await
        .map_err(|e| format!("Qwen transcribe request failed: {}", e))?;

    let status = resp.status();
    let body = resp
        .text()
        .await
        .map_err(|e| format!("Failed to read Qwen response: {}", e))?;

    if !status.is_success() {
        return Err(format!("Qwen API error ({}): {}", status, body));
    }

    let chat_resp: ChatCompletionResponse =
        serde_json::from_str(&body).map_err(|e| format!("Failed to parse Qwen response: {}", e))?;

    let text = chat_resp
        .choices
        .as_ref()
        .and_then(|choices| choices.first())
        .and_then(|choice| choice.message.as_ref())
        .and_then(|msg| msg.content.as_ref())
        .ok_or_else(|| "No text in Qwen response".to_string())?;

    Ok(text.trim().to_string())
}
