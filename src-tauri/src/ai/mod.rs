pub mod types;
pub mod retry;
pub mod gemini;
pub mod openai_compat;
pub mod mimo;
pub mod longcat;
pub mod prompt;
pub mod models;

use crate::config::types::AppConfig;
use std::path::Path;

/// Transcribe audio using the configured provider.
pub async fn transcribe(
    client: &reqwest::Client,
    audio_base64: &str,
    config: &AppConfig,
    prompts_dir: &Path,
) -> Result<String, String> {
    let resolved = models::resolve_model(config, &config.transcribe.model_id)?;
    let system_prompt = prompt::build_transcribe_prompt(config, prompts_dir);

    match resolved.protocol.as_str() {
        "gemini" => {
            gemini::transcribe(
                client,
                audio_base64,
                &system_prompt,
                &resolved.api_key,
                &resolved.model,
                &resolved.base_url,
                &config.transcribe.thinking,
            )
            .await
        }
        "qwen-omni" => {
            openai_compat::qwen_omni_transcribe(
                client,
                audio_base64,
                &system_prompt,
                &resolved.api_key,
                &resolved.model,
                &resolved.base_url,
            )
            .await
        }
        "mimo" => {
            mimo::transcribe(
                client,
                audio_base64,
                &system_prompt,
                &resolved.api_key,
                &resolved.model,
                &resolved.base_url,
            )
            .await
        }
        "longcat" => {
            longcat::transcribe(
                client,
                audio_base64,
                &system_prompt,
                &resolved.api_key,
                &resolved.model,
                &resolved.base_url,
            )
            .await
        }
        _ => {
            openai_compat::transcribe(
                client,
                audio_base64,
                &system_prompt,
                &resolved.api_key,
                &resolved.model,
                &resolved.base_url,
            )
            .await
        }
    }
}

/// Extract text from an image using the configured provider.
pub async fn extract_text(
    client: &reqwest::Client,
    image_base64: &str,
    config: &AppConfig,
    prompts_dir: &Path,
    template_id: &str,
) -> Result<String, String> {
    let model_id = config.extract.model_id.as_deref().unwrap_or(&config.transcribe.model_id);
    let resolved = models::resolve_model(config, model_id)?;
    let thinking = config.extract.thinking.as_ref().unwrap_or(&config.transcribe.thinking);
    let system_prompt = prompt::build_extract_prompt(config, prompts_dir, template_id);

    match resolved.protocol.as_str() {
        "gemini" => {
            gemini::extract_text(
                client,
                image_base64,
                &system_prompt,
                &resolved.api_key,
                &resolved.model,
                &resolved.base_url,
                thinking,
            )
            .await
        }
        "qwen-omni" => {
            openai_compat::qwen_omni_extract_text(
                client,
                image_base64,
                &system_prompt,
                &resolved.api_key,
                &resolved.model,
                &resolved.base_url,
            )
            .await
        }
        "mimo" => {
            mimo::extract_text(
                client,
                image_base64,
                &system_prompt,
                &resolved.api_key,
                &resolved.model,
                &resolved.base_url,
            )
            .await
        }
        "longcat" => {
            longcat::extract_text(
                client,
                image_base64,
                &system_prompt,
                &resolved.api_key,
                &resolved.model,
                &resolved.base_url,
            )
            .await
        }
        _ => {
            openai_compat::extract_text(
                client,
                image_base64,
                &system_prompt,
                &resolved.api_key,
                &resolved.model,
                &resolved.base_url,
            )
            .await
        }
    }
}

/// Optimize text using the configured provider.
pub async fn optimize(
    client: &reqwest::Client,
    text: &str,
    config: &AppConfig,
    prompts_dir: &Path,
    template_id: &str,
) -> Result<String, String> {
    let system_prompt = prompt::load_template_prompt(
        &config.voice_templates.templates,
        template_id,
        prompts_dir,
    );
    if system_prompt.is_empty() {
        return Ok(text.to_string());
    }
    let system_prompt = prompt::wrap_document("text-optimize", &system_prompt);

    let resolved = models::resolve_model(config, &config.voice_templates.model_id)?;

    match resolved.protocol.as_str() {
        "gemini" => {
            gemini::optimize(
                client,
                text,
                &system_prompt,
                &resolved.api_key,
                &resolved.model,
                &resolved.base_url,
                &config.voice_templates.thinking,
            )
            .await
        }
        "qwen-omni" => {
            openai_compat::qwen_omni_optimize(
                client,
                text,
                &system_prompt,
                &resolved.api_key,
                &resolved.model,
                &resolved.base_url,
            )
            .await
        }
        "mimo" => {
            mimo::optimize(
                client,
                text,
                &system_prompt,
                &resolved.api_key,
                &resolved.model,
                &resolved.base_url,
            )
            .await
        }
        "longcat" => {
            longcat::optimize(
                client,
                text,
                &system_prompt,
                &resolved.api_key,
                &resolved.model,
                &resolved.base_url,
            )
            .await
        }
        _ => {
            openai_compat::optimize(
                client,
                text,
                &system_prompt,
                &resolved.api_key,
                &resolved.model,
                &resolved.base_url,
            )
            .await
        }
    }
}
