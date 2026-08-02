use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Mutex;

use axum::body::Bytes;
use axum::extract::{DefaultBodyLimit, Extension, Query, Request, State};
use axum::http::{header, StatusCode};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::post;
use axum::Router;
use base64::Engine as _;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, State as TauriState};
use tokio_util::sync::CancellationToken;

use crate::audio::input::{normalize_audio, validate_content_type};
use crate::config::types::{AppConfig, LocalApiConfig};
use crate::config::ConfigManager;
use crate::task;

const MAX_REQUEST_BYTES: usize = 128 * 1024 * 1024;

#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct LocalApiStatus {
    pub running: bool,
    pub port: Option<u16>,
    pub error: Option<String>,
}

struct RunningServer {
    port: u16,
    shutdown: CancellationToken,
}

pub struct LocalApiManager {
    running: tokio::sync::Mutex<Option<RunningServer>>,
    status: Mutex<LocalApiStatus>,
}

impl LocalApiManager {
    pub fn new() -> Self {
        Self {
            running: tokio::sync::Mutex::new(None),
            status: Mutex::new(LocalApiStatus::default()),
        }
    }

    pub fn status(&self) -> LocalApiStatus {
        self.status
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .clone()
    }

    pub async fn configure(&self, app: AppHandle, config: &LocalApiConfig) -> Result<(), String> {
        let mut running = self.running.lock().await;

        if !config.enabled {
            if let Some(server) = running.take() {
                server.shutdown.cancel();
            }
            self.set_status(&app, LocalApiStatus::default());
            return Ok(());
        }

        if running
            .as_ref()
            .is_some_and(|server| server.port == config.port)
        {
            self.set_status(
                &app,
                LocalApiStatus {
                    running: true,
                    port: Some(config.port),
                    error: None,
                },
            );
            return Ok(());
        }

        let address = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), config.port);
        let listener = match tokio::net::TcpListener::bind(address).await {
            Ok(listener) => listener,
            Err(error) => {
                let message = format!("无法监听127.0.0.1:{}：{}", config.port, error);
                let previous_port = running.as_ref().map(|server| server.port);
                self.set_status(
                    &app,
                    LocalApiStatus {
                        running: previous_port.is_some(),
                        port: previous_port,
                        error: Some(message.clone()),
                    },
                );
                return Err(message);
            }
        };

        let shutdown = CancellationToken::new();
        let shutdown_signal = shutdown.clone();
        let state = ApiState { app: app.clone() };
        let router = Router::new()
            .route("/transcribe", post(transcribe))
            .route_layer(middleware::from_fn_with_state(
                state.clone(),
                reserve_task_slot,
            ))
            .layer(DefaultBodyLimit::max(MAX_REQUEST_BYTES))
            .with_state(state);

        tauri::async_runtime::spawn(async move {
            let result = axum::serve(listener, router)
                .with_graceful_shutdown(shutdown_signal.cancelled_owned())
                .await;
            if let Err(error) = result {
                eprintln!("[LocalApi] HTTP server stopped with error: {}", error);
            }
        });

        if let Some(previous) = running.replace(RunningServer {
            port: config.port,
            shutdown,
        }) {
            previous.shutdown.cancel();
        }
        self.set_status(
            &app,
            LocalApiStatus {
                running: true,
                port: Some(config.port),
                error: None,
            },
        );
        Ok(())
    }

    fn set_status(&self, app: &AppHandle, status: LocalApiStatus) {
        *self
            .status
            .lock()
            .unwrap_or_else(|error| error.into_inner()) = status.clone();
        let _ = app.emit("local-api-status", status);
    }
}

#[derive(Clone)]
struct ApiState {
    app: AppHandle,
}

#[derive(Clone)]
struct RequestTaskToken(CancellationToken);

#[derive(Clone)]
struct RequestAudioType(String);

#[derive(Deserialize)]
struct TranscribeQuery {
    template: Option<String>,
}

async fn transcribe(
    State(state): State<ApiState>,
    Extension(RequestTaskToken(token)): Extension<RequestTaskToken>,
    Extension(RequestAudioType(content_type)): Extension<RequestAudioType>,
    Query(query): Query<TranscribeQuery>,
    body: Bytes,
) -> Response {
    let config = state.app.state::<ConfigManager>().get();
    let template_id = match resolve_template(&config, query.template.as_deref()) {
        Ok(template_id) => template_id,
        Err(error) => return text_response(StatusCode::BAD_REQUEST, error),
    };
    let max_duration_seconds = config.general.max_recording_seconds;
    let audio_bytes = body.to_vec();
    drop(body);
    let conversion_token = token.clone();
    let audio_base64 = match tokio::task::spawn_blocking(move || {
        let normalized = normalize_audio(
            audio_bytes,
            &content_type,
            max_duration_seconds,
            &conversion_token,
        )?;
        encode_base64_with_cancel(normalized.flac, &conversion_token)
    })
    .await
    {
        Ok(Ok(audio_base64)) => audio_base64,
        Ok(Err(error)) => return text_response(StatusCode::BAD_REQUEST, error),
        Err(error) => {
            return text_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("音频转换任务失败：{}", error),
            )
        }
    };
    let result = task::run_silent_pipeline(&state.app, audio_base64, token, template_id).await;

    match result {
        Ok(text) => text_response(StatusCode::OK, text),
        Err(error) if error.contains("timed out") => {
            text_response(StatusCode::GATEWAY_TIMEOUT, error)
        }
        Err(error) => text_response(StatusCode::BAD_GATEWAY, error),
    }
}

fn encode_base64_with_cancel(
    bytes: Vec<u8>,
    cancellation: &CancellationToken,
) -> Result<String, String> {
    const INPUT_CHUNK_BYTES: usize = 3 * 4096;

    let mut encoded = String::with_capacity(bytes.len().saturating_mul(4) / 3 + 4);
    for chunk in bytes.chunks(INPUT_CHUNK_BYTES) {
        if cancellation.is_cancelled() {
            return Err("请求已取消".to_string());
        }
        base64::engine::general_purpose::STANDARD.encode_string(chunk, &mut encoded);
    }
    Ok(encoded)
}

async fn reserve_task_slot(
    State(state): State<ApiState>,
    mut request: Request,
    next: Next,
) -> Response {
    let content_type = match request
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
    {
        Some(content_type) => content_type.to_string(),
        None => return text_response(StatusCode::UNSUPPORTED_MEDIA_TYPE, "缺少Content-Type请求头"),
    };
    if let Err(error) = validate_content_type(&content_type) {
        return text_response(StatusCode::UNSUPPORTED_MEDIA_TYPE, error);
    }
    let lease = match task::start_external_task(&state.app) {
        Ok(lease) => lease,
        Err(error) => return text_response(StatusCode::TOO_MANY_REQUESTS, error),
    };
    request
        .extensions_mut()
        .insert(RequestTaskToken(lease.token()));
    request
        .extensions_mut()
        .insert(RequestAudioType(content_type));
    let response = next.run(request).await;
    lease.finish();
    response
}

fn text_response(status: StatusCode, body: impl Into<String>) -> Response {
    (
        status,
        [(header::CONTENT_TYPE, "text/plain; charset=utf-8")],
        body.into(),
    )
        .into_response()
}

fn resolve_template(config: &AppConfig, requested: Option<&str>) -> Result<Option<String>, String> {
    match requested {
        Some("raw") => Ok(None),
        Some(template_id)
            if config
                .voice_templates
                .templates
                .iter()
                .any(|template| template.id == template_id) =>
        {
            Ok(Some(template_id.to_string()))
        }
        Some(template_id) => Err(format!("未知语音模板：{}", template_id)),
        None => Ok((!config.general.shortcut_template.is_empty())
            .then(|| config.general.shortcut_template.clone())),
    }
}

#[tauri::command]
pub fn get_local_api_status(
    manager: TauriState<'_, LocalApiManager>,
) -> Result<LocalApiStatus, String> {
    Ok(manager.status())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::types::AppConfig;
    use std::sync::Arc;
    use tokio::sync::Notify;

    #[test]
    fn missing_template_uses_the_first_shortcut_binding() {
        let config = AppConfig::default();

        assert_eq!(
            resolve_template(&config, None).unwrap(),
            Some("voice-optimize".to_string())
        );
    }

    #[test]
    fn raw_template_skips_text_optimization() {
        let config = AppConfig::default();

        assert_eq!(resolve_template(&config, Some("raw")), Ok(None));
    }

    #[test]
    fn unknown_template_is_rejected() {
        let config = AppConfig::default();

        assert_eq!(
            resolve_template(&config, Some("missing")),
            Err("未知语音模板：missing".to_string())
        );
    }

    #[test]
    fn cancelled_base64_conversion_stops_before_encoding() {
        let cancellation = CancellationToken::new();
        cancellation.cancel();

        let error = encode_base64_with_cancel(b"audio".to_vec(), &cancellation).unwrap_err();

        assert_eq!(error, "请求已取消");
    }

    #[tokio::test]
    async fn client_disconnect_drops_the_request_future() {
        #[derive(Clone)]
        struct Probe {
            started: Arc<Notify>,
            cancelled: Arc<Notify>,
        }

        struct NotifyOnDrop(Arc<Notify>);

        impl Drop for NotifyOnDrop {
            fn drop(&mut self) {
                self.0.notify_one();
            }
        }

        async fn pending_request(State(probe): State<Probe>) -> &'static str {
            let _cancel_on_drop = NotifyOnDrop(probe.cancelled.clone());
            probe.started.notify_one();
            std::future::pending::<()>().await;
            "unreachable"
        }

        let probe = Probe {
            started: Arc::new(Notify::new()),
            cancelled: Arc::new(Notify::new()),
        };
        let listener = tokio::net::TcpListener::bind((Ipv4Addr::LOCALHOST, 0))
            .await
            .unwrap();
        let address = listener.local_addr().unwrap();
        let server_probe = probe.clone();
        let server = tokio::spawn(async move {
            axum::serve(
                listener,
                Router::new()
                    .route("/probe", post(pending_request))
                    .with_state(server_probe),
            )
            .await
            .unwrap();
        });
        let client = tokio::net::TcpStream::connect(address).await.unwrap();
        client.writable().await.unwrap();
        client
            .try_write(b"POST /probe HTTP/1.1\r\nHost: localhost\r\nContent-Length: 0\r\n\r\n")
            .unwrap();
        tokio::time::timeout(std::time::Duration::from_secs(1), probe.started.notified())
            .await
            .unwrap();

        drop(client);

        tokio::time::timeout(
            std::time::Duration::from_secs(1),
            probe.cancelled.notified(),
        )
        .await
        .unwrap();
        server.abort();
    }
}
