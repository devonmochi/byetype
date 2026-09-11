use reqwest::Client;

use super::types::{ChatCompletionRequest, ChatCompletionResponse};

/// Bearer 鉴权头，DeepSeek / Qwen Omni / OpenRouter 都用这套。
pub(crate) fn bearer(api_key: &str) -> [(&'static str, String); 1] {
    [("Authorization", format!("Bearer {}", api_key))]
}

/// 发一份 OpenAI 兼容的聊天请求：装请求头、发出去、判状态码、读回响应体。
/// 返回响应体原文，调用方自己按 JSON 或 SSE 解析。label 是错误文案里的服务商名。
pub(crate) async fn post(
    client: &Client,
    url: &str,
    request: &ChatCompletionRequest,
    headers: &[(&str, String)],
    label: &str,
) -> Result<String, String> {
    let mut builder = client.post(url).json(request);
    for (name, value) in headers {
        builder = builder.header(*name, value);
    }

    let resp = builder
        .send()
        .await
        .map_err(|e| format!("{} request failed: {}", label, e))?;

    let status = resp.status();
    let body = resp
        .text()
        .await
        .map_err(|e| format!("Failed to read {} response: {}", label, e))?;

    if !status.is_success() {
        return Err(format!("{} API error ({}): {}", label, status, body));
    }

    Ok(body)
}

/// 非流式请求，直接把响应体解析成 ChatCompletionResponse。
pub(crate) async fn chat(
    client: &Client,
    url: &str,
    request: &ChatCompletionRequest,
    headers: &[(&str, String)],
    label: &str,
) -> Result<ChatCompletionResponse, String> {
    let body = post(client, url, request, headers, label).await?;
    serde_json::from_str(&body).map_err(|e| format!("Failed to parse {} response: {}", label, e))
}
