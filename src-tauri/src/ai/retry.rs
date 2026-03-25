use std::future::Future;
use std::time::Duration;

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
                if attempt >= max_retries {
                    return Err(e);
                }
                eprintln!("[AI] Attempt {} failed: {}, retrying...", attempt + 1, e);
                on_retry(attempt + 1);
            }
            Err(_) => {
                if attempt >= max_retries {
                    return Err("Request timed out".to_string());
                }
                eprintln!("[AI] Attempt {} timed out, retrying...", attempt + 1);
                on_retry(attempt + 1);
            }
        }
    }
    Err("All retries exhausted".to_string())
}
