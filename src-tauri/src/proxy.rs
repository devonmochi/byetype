use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::str::FromStr;

#[derive(Debug, Deserialize)]
pub struct ProxyRequest {
    pub url: String,
    pub method: String,
    pub headers: HashMap<String, String>,
    pub body: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ProxyResponse {
    pub status: u16,
    pub headers: HashMap<String, String>,
    pub body: String,
}

pub async fn forward_request(proxy_url: &str, req: ProxyRequest) -> Result<ProxyResponse, String> {
    let client = if proxy_url.is_empty() {
        reqwest::Client::new()
    } else {
        let proxy = reqwest::Proxy::all(proxy_url)
            .map_err(|e| format!("Invalid proxy URL: {}", e))?;
        reqwest::Client::builder()
            .proxy(proxy)
            .build()
            .map_err(|e| format!("Failed to build proxy client: {}", e))?
    };

    let method = reqwest::Method::from_str(&req.method.to_uppercase())
        .map_err(|e| format!("Invalid method: {}", e))?;

    let mut headers = HeaderMap::new();
    for (k, v) in &req.headers {
        if let (Ok(name), Ok(val)) = (HeaderName::from_str(k), HeaderValue::from_str(v)) {
            headers.insert(name, val);
        }
    }

    let mut request = client.request(method, &req.url).headers(headers);
    if let Some(body) = req.body {
        request = request.body(body);
    }

    let response = request.send().await
        .map_err(|e| format!("Request failed: {}", e))?;

    let status = response.status().as_u16();
    let resp_headers: HashMap<String, String> = response.headers().iter()
        .filter_map(|(k, v)| v.to_str().ok().map(|val| (k.to_string(), val.to_string())))
        .collect();
    let body = response.text().await
        .map_err(|e| format!("Failed to read response: {}", e))?;

    Ok(ProxyResponse { status, headers: resp_headers, body })
}
