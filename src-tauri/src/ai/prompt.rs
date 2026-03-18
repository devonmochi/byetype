use std::path::Path;

use crate::config::types::AppConfig;

pub fn load_prompt(file_path: &str) -> String {
    if file_path.is_empty() {
        return String::new();
    }
    std::fs::read_to_string(file_path).unwrap_or_default()
}

pub fn wrap_document(name: &str, content: &str) -> String {
    if content.is_empty() {
        return String::new();
    }
    format!("<document name=\"{}\">\n{}\n</document>", name, content)
}

pub fn resolve_prompt_path(custom: &str, builtin: &str) -> String {
    if !custom.is_empty() {
        custom.to_string()
    } else {
        builtin.to_string()
    }
}

pub fn build_transcribe_prompt(config: &AppConfig, prompts_dir: &Path) -> String {
    let agent_path = resolve_prompt_path(
        &config.transcribe.prompts.agent,
        &prompts_dir.join("agent.md").to_string_lossy(),
    );
    let vocabulary_path = resolve_prompt_path(
        &config.transcribe.prompts.vocabulary,
        &prompts_dir.join("vocabulary.md").to_string_lossy(),
    );
    let rules_path = resolve_prompt_path(
        &config.transcribe.prompts.rules,
        &prompts_dir.join("rules.md").to_string_lossy(),
    );

    let agent_content = load_prompt(&agent_path);
    let vocabulary_content = load_prompt(&vocabulary_path);
    let rules_content = load_prompt(&rules_path);

    let parts: Vec<String> = [
        wrap_document("agent", &agent_content),
        wrap_document("vocabulary", &vocabulary_content),
        wrap_document("rules", &rules_content),
    ]
    .into_iter()
    .filter(|s| !s.is_empty())
    .collect();

    parts.join("\n\n")
}

pub fn load_optimize_prompt(config: &AppConfig, prompts_dir: &Path) -> String {
    let prompt_path = resolve_prompt_path(
        &config.optimize.prompt,
        &prompts_dir.join("text-optimize.md").to_string_lossy(),
    );

    let content = load_prompt(&prompt_path);
    wrap_document("text-optimize", &content)
}
