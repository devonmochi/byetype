use serde::{Deserialize, Serialize};

/// 一次模型调用的 Token 用量。服务商未返回时保持 0，调用次数仍计入统计。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TokenUsage {
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
}

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
#[serde(rename_all = "camelCase")]
pub struct GeminiResponse {
    pub candidates: Option<Vec<GeminiCandidate>>,
    #[serde(default)]
    pub usage_metadata: Option<GeminiUsageMetadata>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GeminiUsageMetadata {
    #[serde(default)]
    pub prompt_token_count: Option<u64>,
    #[serde(default)]
    pub candidates_token_count: Option<u64>,
    #[serde(default)]
    pub total_token_count: Option<u64>,
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
    pub provider: Option<OpenRouterProvider>,
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
pub struct OpenRouterProvider {
    pub order: Vec<String>,
    pub allow_fallbacks: bool,
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
    #[serde(default)]
    pub usage: Option<ChatUsage>,
}

/// OpenAI 兼容接口的 usage 字段。各服务商字段一致（snake_case）。
#[derive(Deserialize, Clone, Copy)]
pub struct ChatUsage {
    #[serde(default)]
    pub prompt_tokens: Option<u64>,
    #[serde(default)]
    pub completion_tokens: Option<u64>,
}

impl TokenUsage {
    /// Gemini 的 totalTokenCount 包含思考 Token，输出 = total - prompt；
    /// 缺失 total 时退回 candidatesTokenCount。
    pub fn from_gemini(meta: &GeminiUsageMetadata) -> Self {
        let prompt = meta.prompt_token_count.unwrap_or(0);
        let completion = match (meta.total_token_count, meta.candidates_token_count) {
            (Some(total), _) => total.saturating_sub(prompt),
            (None, Some(candidates)) => candidates,
            (None, None) => 0,
        };
        Self {
            prompt_tokens: prompt,
            completion_tokens: completion,
        }
    }

    pub fn from_chat(usage: &ChatUsage) -> Self {
        Self {
            prompt_tokens: usage.prompt_tokens.unwrap_or(0),
            completion_tokens: usage.completion_tokens.unwrap_or(0),
        }
    }
}

#[derive(Deserialize)]
pub struct ChatChoice {
    pub message: Option<ChatResponseMessage>,
}

#[derive(Deserialize)]
pub struct ChatResponseMessage {
    pub content: Option<String>,
}

// === SSE streaming types (Qwen Omni) ===

#[derive(Serialize)]
pub struct StreamOptions {
    pub include_usage: bool,
}

#[derive(Deserialize)]
pub struct StreamChunk {
    pub choices: Option<Vec<StreamChunkChoice>>,
    /// include_usage 开启时，最后一个 chunk 会携带 usage（其余 chunk 为 null）。
    #[serde(default)]
    pub usage: Option<ChatUsage>,
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
    fn parses_gemini_usage_metadata() {
        let resp: GeminiResponse = serde_json::from_str(
            r#"{"candidates":[{"content":{"parts":[{"text":"hi"}]}}],
                "usageMetadata":{"promptTokenCount":100,"candidatesTokenCount":40,"totalTokenCount":180}}"#,
        )
        .unwrap();
        let usage = TokenUsage::from_gemini(resp.usage_metadata.as_ref().unwrap());
        assert_eq!(usage.prompt_tokens, 100);
        // total 180 - prompt 100 = 80（含思考 Token）
        assert_eq!(usage.completion_tokens, 80);
    }

    #[test]
    fn gemini_usage_without_total_falls_back_to_candidates() {
        let resp: GeminiResponse = serde_json::from_str(
            r#"{"candidates":[],"usageMetadata":{"promptTokenCount":10,"candidatesTokenCount":7}}"#,
        )
        .unwrap();
        let usage = TokenUsage::from_gemini(resp.usage_metadata.as_ref().unwrap());
        assert_eq!((usage.prompt_tokens, usage.completion_tokens), (10, 7));
    }

    #[test]
    fn parses_chat_usage_from_stream_chunk() {
        let chunk: StreamChunk = serde_json::from_str(
            r#"{"choices":[],"usage":{"prompt_tokens":12,"completion_tokens":34,"total_tokens":46}}"#,
        )
        .unwrap();
        let usage = TokenUsage::from_chat(chunk.usage.as_ref().unwrap());
        assert_eq!((usage.prompt_tokens, usage.completion_tokens), (12, 34));
    }

    #[test]
    fn missing_usage_defaults_to_zero() {
        let resp: ChatCompletionResponse =
            serde_json::from_str(r#"{"choices":[{"message":{"content":"hi"}}]}"#).unwrap();
        assert!(resp.usage.is_none());
    }

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
            provider: None,
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
