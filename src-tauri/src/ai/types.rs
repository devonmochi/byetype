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
}

#[derive(Serialize)]
pub struct AudioData {
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub audio_type: Option<String>,
    pub data: String,
    pub format: String,
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
}
