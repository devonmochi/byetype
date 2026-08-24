use serde::{Deserialize, Serialize};

// === Gemini types ===

#[derive(Serialize)]
pub struct GeminiRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_instruction: Option<GeminiContent>,
    pub contents: Vec<GeminiContent>,
    #[serde(rename = "generationConfig", skip_serializing_if = "Option::is_none")]
    pub generation_config: Option<GeminiGenerationConfig>,
}

#[derive(Serialize)]
pub struct GeminiContent {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    pub parts: Vec<GeminiPart>,
}

#[derive(Serialize)]
#[serde(untagged)]
pub enum GeminiPart {
    Text { text: String },
    InlineData {
        #[serde(rename = "inlineData")]
        inline_data: GeminiInlineData,
    },
}

#[derive(Serialize)]
pub struct GeminiInlineData {
    #[serde(rename = "mimeType")]
    pub mime_type: String,
    pub data: String,
}

#[derive(Serialize)]
pub struct GeminiGenerationConfig {
    #[serde(rename = "thinkingConfig", skip_serializing_if = "Option::is_none")]
    pub thinking_config: Option<GeminiThinkingConfig>,
}

#[derive(Serialize)]
pub struct GeminiThinkingConfig {
    pub include_thoughts: bool,
    #[serde(rename = "thinkingLevel")]
    pub thinking_level: String,
}

#[derive(Deserialize)]
pub struct GeminiResponse {
    pub candidates: Option<Vec<GeminiCandidate>>,
}

#[derive(Deserialize)]
pub struct GeminiCandidate {
    pub content: Option<GeminiResponseContent>,
}

#[derive(Deserialize)]
pub struct GeminiResponseContent {
    pub parts: Option<Vec<GeminiResponsePart>>,
}

#[derive(Deserialize)]
pub struct GeminiResponsePart {
    pub text: Option<String>,
}

// === OpenAI-compat types (Qwen + optimize) ===

#[derive(Serialize)]
pub struct ChatCompletionRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modalities: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_modalities: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream_options: Option<StreamOptions>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking: Option<ThinkingParam>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_effort: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning: Option<OpenRouterReasoning>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chat_template_kwargs: Option<serde_json::Value>,
}

#[derive(Serialize)]
pub struct ThinkingParam {
    #[serde(rename = "type")]
    pub thinking_type: String,
}

#[derive(Serialize)]
pub struct OpenRouterReasoning {
    pub effort: String,
}

#[derive(Serialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: ChatContent,
}

#[derive(Serialize)]
#[serde(untagged)]
pub enum ChatContent {
    Text(String),
    Parts(Vec<ChatContentPart>),
}

#[derive(Serialize)]
#[serde(tag = "type")]
pub enum ChatContentPart {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "input_audio")]
    InputAudio { input_audio: AudioData },
    #[serde(rename = "audio_url")]
    AudioUrl { audio_url: AudioUrlData },
    #[serde(rename = "image_url")]
    ImageUrl { image_url: ImageUrlData },
}

#[derive(Serialize)]
pub struct AudioData {
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub audio_type: Option<String>,
    pub data: String,
    pub format: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sample_rate: Option<u32>,
}

#[derive(Serialize)]
pub struct AudioUrlData {
    pub url: String,
}

#[derive(Serialize)]
pub struct ImageUrlData {
    pub url: String,
}

#[derive(Deserialize)]
pub struct ChatCompletionResponse {
    pub choices: Option<Vec<ChatChoice>>,
}

#[derive(Deserialize)]
pub struct ChatChoice {
    pub message: Option<ChatResponseMessage>,
}

#[derive(Deserialize)]
pub struct ChatResponseMessage {
    pub content: Option<String>,
    pub reasoning_content: Option<String>,
}

// === SSE streaming types (Qwen Omni) ===

#[derive(Serialize)]
pub struct StreamOptions {
    pub include_usage: bool,
}

#[derive(Deserialize)]
pub struct StreamChunk {
    pub choices: Option<Vec<StreamChunkChoice>>,
}

#[derive(Deserialize)]
pub struct StreamChunkChoice {
    pub delta: Option<StreamDelta>,
}

#[derive(Deserialize)]
pub struct StreamDelta {
    pub content: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serializes_audio_url_content_part_as_data_uri() {
        let part = ChatContentPart::AudioUrl {
            audio_url: AudioUrlData {
                url: "data:audio/flac;base64,ZmFrZQ==".to_string(),
            },
        };

        assert_eq!(
            serde_json::to_value(part).unwrap(),
            serde_json::json!({
                "type": "audio_url",
                "audio_url": {
                    "url": "data:audio/flac;base64,ZmFrZQ=="
                }
            })
        );
    }

    #[test]
    fn serializes_chat_template_kwargs_as_an_object() {
        let request = ChatCompletionRequest {
            model: "test-model".to_string(),
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
            chat_template_kwargs: Some(serde_json::json!({
                "enable_thinking": false,
                "custom_flag": "value"
            })),
        };

        let value = serde_json::to_value(request).unwrap();

        assert_eq!(
            value["chat_template_kwargs"],
            serde_json::json!({"enable_thinking": false, "custom_flag": "value"})
        );
    }
}
