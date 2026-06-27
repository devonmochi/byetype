use std::future::Future;
use std::time::Duration;

/// 判断错误字符串是否表示不应重试的 4xx 客户端错误。
///
/// 各 provider 的错误统一格式为 `"<Provider> API error (<status>): <body>"`，
/// 其中 `<status>` 为 HTTP 状态码数字。408（请求超时）与 429（限流）属于
/// 可重试的 4xx，其余 4xx（如 400/401/403/404/422）重试无意义，直接返回。
/// 解析失败时返回 false，保持原有重试行为。
fn is_non_retryable_client_error(err: &str) -> bool {
    // 取第一个 '(' 与其后首个 ')' 之间的内容作为状态码文本。
    let start = match err.find('(') {
        Some(i) => i + 1,
        None => return false,
    };
    let end = match err[start..].find(')') {
        Some(j) => start + j,
        None => return false,
    };
    let code_text = &err[start..end];
    let code: u16 = match code_text.trim().parse() {
        Ok(c) => c,
        Err(_) => return false,
    };
    if code >= 400 && code < 500 && code != 408 && code != 429 {
        eprintln!("[AI] Detected non-retryable {} error", code);
        return true;
    }
    false
}

pub async fn with_retry<F, Fut, T, R>(
    f: F,
    max_retries: u32,
    timeout_secs: u32,
    on_retry: R,
) -> Result<T, String>
where
    F: Fn() -> Fut,
    Fut: Future<Output = Result<T, String>>,
    R: Fn(u32),
{
    let timeout_duration = Duration::from_secs(timeout_secs as u64);

    for attempt in 0..=max_retries {
        match tokio::time::timeout(timeout_duration, f()).await {
            Ok(Ok(result)) => return Ok(result),
            Ok(Err(e)) => {
                // 4xx 客户端错误（除 408 Request Timeout / 429 Too Many Requests 外）
                // 重试不会成功，只会浪费配额、放大延迟并可能触发限流，直接返回。
                if is_non_retryable_client_error(&e) {
                    eprintln!("[AI] Attempt {} failed with non-retryable error: {}, not retrying", attempt + 1, e);
                    return Err(e);
                }
                if attempt >= max_retries {
                    return Err(e);
                }
                eprintln!("[AI] Attempt {} failed: {}, retrying...", attempt + 1, e);
                on_retry(attempt + 1);
                // 指数退避：避免对 429 限流与 5xx/网络抖动立即重试，
                // 防止白白耗尽重试次数并放大服务端压力。
                let backoff = Duration::from_millis(500u64.saturating_mul(1u64 << attempt.min(63)));
                tokio::time::sleep(backoff).await;
            }
            Err(_) => {
                if attempt >= max_retries {
                    return Err("Request timed out".to_string());
                }
                eprintln!("[AI] Attempt {} timed out, retrying...", attempt + 1);
                on_retry(attempt + 1);
                // 超时分支同样应用指数退避。
                let backoff = Duration::from_millis(500u64.saturating_mul(1u64 << attempt.min(63)));
                tokio::time::sleep(backoff).await;
            }
        }
    }
    Err("All retries exhausted".to_string())
}
