use reqwest::Client;

use super::types::*;
use crate::config::types::ThinkingConfig;

pub fn build_thinking_config(
    model: &str,
    thinking: &ThinkingConfig,
) -> Option<GeminiGenerationConfig> {
    // Gemini 3.7/3.8 系列强制思考且不支持 minimal;关闭开关时也要显式发 low,
    // 否则省略参数会让 Google 按默认 medium 档思考,比开开关的 LOW 更慢。
    if !thinking.enabled {
        if forced_thinking_model(model) {
            return Some(GeminiGenerationConfig {
                thinking_config: Some(GeminiThinkingConfig {
                    include_thoughts: false,
                    thinking_level: "low".to_string(),
                }),
            });
        }
        return None;
    }
    // Gemini 的 thinkingLevel 只接受小写值 (minimal/low/medium/high)。
    // ThinkingConfig.level 在前端以大写存储,需要转小写,与 openai_compat.rs 保持一致。
    let level = thinking.level.trim().to_lowercase();
    let mut thinking_level = if level.is_empty() {
        "medium".to_string()
    } else {
        level
    };
    // Gemini 3.7/3.8 系列(如 gemini-3.8-flash)不支持 minimal 档位,官方 API 会返回错误,降级为 low。
    if thinking_level == "minimal" && forced_thinking_model(model) {
        thinking_level = "low".to_string();
    }
    Some(GeminiGenerationConfig {
        thinking_config: Some(GeminiThinkingConfig {
            include_thoughts: false,
            thinking_level,
        }),
    })
}

/// Gemini 3.7/3.8 系列强制思考,直连时不支持 minimal,最低可用档位是 low。
fn forced_thinking_model(model: &str) -> bool {
    model.contains("gemini-3.7") || model.contains("gemini-3.8")
}

pub async fn transcribe(
    client: &Client,
    audio_base64: &str,
    system_prompt: &str,
    api_key: &str,
    model: &str,
    base_url: &str,
    thinking: &ThinkingConfig,
) -> Result<String, String> {
    let url = format!(
        "{}/v1beta/models/{}:generateContent?key={}",
        base_url.trim_end_matches('/'),
        model,
        api_key
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
                    mime_type: "audio/flac".to_string(),
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
    base_url: &str,
    thinking: &ThinkingConfig,
) -> Result<String, String> {
    let url = format!(
        "{}/v1beta/models/{}:generateContent?key={}",
        base_url.trim_end_matches('/'),
        model,
        api_key
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

    let user_content = format!("<voice-input>\n{}\n</voice-input>", text);

    let request = GeminiRequest {
        system_instruction,
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

pub async fn test_connectivity(
    client: &Client,
    api_key: &str,
    model: &str,
    base_url: &str,
) -> Result<(), String> {
    let url = format!(
        "{}/v1beta/models/{}:generateContent?key={}",
        base_url.trim_end_matches('/'),
        model,
        api_key
    );

    let request = GeminiRequest {
        system_instruction: None,
        contents: vec![GeminiContent {
            role: Some("user".to_string()),
            parts: vec![GeminiPart::Text {
                text: "hi".to_string(),
            }],
        }],
        generation_config: None,
    };

    let resp = client
        .post(&url)
        .json(&request)
        .send()
        .await
        .map_err(|e| format!("Gemini connectivity test failed: {}", e))?;

    let status = resp.status();
    if !status.is_success() {
        let body = resp
            .text()
            .await
            .map_err(|e| format!("Failed to read Gemini response: {}", e))?;
        return Err(format!("Gemini API error ({}): {}", status, body));
    }

    Ok(())
}

pub async fn extract_text(
    client: &Client,
    image_base64: &str,
    system_prompt: &str,
    api_key: &str,
    model: &str,
    base_url: &str,
    thinking: &ThinkingConfig,
) -> Result<String, String> {
    let url = format!(
        "{}/v1beta/models/{}:generateContent?key={}",
        base_url.trim_end_matches('/'),
        model,
        api_key
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
                    mime_type: "image/png".to_string(),
                    data: image_base64.to_string(),
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
        .map_err(|e| format!("Gemini extract_text request failed: {}", e))?;

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

#[cfg(test)]
mod tests {
    use super::*;

    fn thinking(level: &str) -> ThinkingConfig {
        ThinkingConfig { enabled: true, budget: 1024, level: level.to_string() }
    }

    #[test]
    fn minimal_downgrades_to_low_on_gemini_3_7() {
        let cfg = build_thinking_config("gemini-3.7-flash", &thinking("MINIMAL"));
        assert_eq!(cfg.unwrap().thinking_config.unwrap().thinking_level, "low");
    }

    #[test]
    fn minimal_downgrades_to_low_on_gemini_3_8() {
        // 3.8 同样不支持 minimal,残留配置降级为 low 而不是让 Google 报 400
        let cfg = build_thinking_config("gemini-3.8-flash", &thinking("MINIMAL"));
        assert_eq!(cfg.unwrap().thinking_config.unwrap().thinking_level, "low");
    }

    #[test]
    fn minimal_stays_on_older_gemini() {
        let cfg = build_thinking_config("gemini-3.1-flash-lite-preview", &thinking("MINIMAL"));
        assert_eq!(cfg.unwrap().thinking_config.unwrap().thinking_level, "minimal");
    }

    #[test]
    fn disabled_still_sends_low_on_gemini_3_7() {
        // 3.7 强制思考:关闭开关必须显式发 low,省略参数会落到默认 medium 更慢
        let off = ThinkingConfig { enabled: false, budget: 1024, level: "LOW".to_string() };
        let cfg = build_thinking_config("gemini-3.7-flash", &off);
        assert_eq!(cfg.unwrap().thinking_config.unwrap().thinking_level, "low");
    }

    #[test]
    fn disabled_still_sends_low_on_gemini_3_8() {
        // 3.8 强制思考:关闭开关发 low,即直连路径的最小可用档位
        let off = ThinkingConfig { enabled: false, budget: 1024, level: "LOW".to_string() };
        let cfg = build_thinking_config("gemini-3.8-flash", &off);
        assert_eq!(cfg.unwrap().thinking_config.unwrap().thinking_level, "low");
    }

    #[test]
    fn disabled_sends_nothing_on_optional_thinking_gemini() {
        let off = ThinkingConfig { enabled: false, budget: 1024, level: "LOW".to_string() };
        assert!(build_thinking_config("gemini-3.5-flash-lite", &off).is_none());
    }
}
