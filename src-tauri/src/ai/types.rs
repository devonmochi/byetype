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
}

#[derive(Serialize)]
pub struct ThinkingParam {
    #[serde(rename = "type")]
    pub thinking_type: String,
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
    #[serde(rename = "image_url")]
    ImageUrl { image_url: ImageUrlData },
    #[serde(rename = "input_image")]
    InputImage { input_image: ImageData },
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
pub struct ImageUrlData {
    pub url: String,
}

#[derive(Serialize)]
pub struct ImageData {
    #[serde(rename = "type")]
    pub image_type: String,
    pub data: String,
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
