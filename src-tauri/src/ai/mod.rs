pub mod types;
pub mod retry;
pub mod gemini;
pub mod openai_compat;
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
        "longcat" => {
            openai_compat::longcat_transcribe(
                client,
                audio_base64,
                &system_prompt,
                &resolved.api_key,
                &resolved.model,
                &resolved.base_url,
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

/// Optimize text using the configured provider.
pub async fn optimize(
    client: &reqwest::Client,
    text: &str,
    config: &AppConfig,
    prompts_dir: &Path,
) -> Result<String, String> {
    if !config.optimize.enabled {
        return Ok(text.to_string());
    }

    let system_prompt = prompt::load_optimize_prompt(config, prompts_dir);
    if system_prompt.is_empty() {
        return Ok(text.to_string());
    }

    let resolved = models::resolve_model(config, &config.optimize.model_id)?;

    match resolved.protocol.as_str() {
        "gemini" => {
            gemini::optimize(
                client,
                text,
                &system_prompt,
                &resolved.api_key,
                &resolved.model,
                &resolved.base_url,
                &config.optimize.thinking,
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
