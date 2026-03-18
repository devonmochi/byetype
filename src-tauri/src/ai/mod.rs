pub mod types;
pub mod retry;
pub mod gemini;
pub mod qwen;
pub mod openai_compat;
pub mod prompt;

use crate::config::types::AppConfig;
use std::path::Path;

/// Transcribe audio using the configured provider.
pub async fn transcribe(
    client: &reqwest::Client,
    audio_base64: &str,
    config: &AppConfig,
    prompts_dir: &Path,
) -> Result<String, String> {
    let system_prompt = prompt::build_transcribe_prompt(config, prompts_dir);

    match config.transcribe.model.as_str() {
        "qwen3-omni-flash" => {
            qwen::transcribe(client, audio_base64, &system_prompt, &config.transcribe.qwen_api_key).await
        }
        model => {
            gemini::transcribe(
                client,
                audio_base64,
                &system_prompt,
                &config.transcribe.gemini_api_key,
                model,
                &config.transcribe.thinking,
            ).await
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

    match config.optimize.optimize_type.as_str() {
        "gemini" => {
            gemini::optimize(
                client,
                text,
                &system_prompt,
                &config.transcribe.gemini_api_key,
                &config.optimize.gemini_model,
                &config.optimize.thinking,
            ).await
        }
        _ => {
            openai_compat::optimize(
                client,
                text,
                &system_prompt,
                &config.optimize.openai_compat,
            ).await
        }
    }
}
