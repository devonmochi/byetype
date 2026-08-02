use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Mutex;

use axum::body::Bytes;
use axum::extract::{DefaultBodyLimit, Query, State};
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::post;
use axum::Router;
use base64::Engine as _;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, State as TauriState};
use tokio_util::sync::CancellationToken;

use crate::audio::input::normalize_audio;
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
                let message = format!("无法监听 127.0.0.1:{}：{}", config.port, error);
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
        let router = Router::new()
            .route("/transcribe", post(transcribe))
            .layer(DefaultBodyLimit::max(MAX_REQUEST_BYTES))
            .with_state(ApiState { app: app.clone() });

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

#[derive(Deserialize)]
struct TranscribeQuery {
    template: Option<String>,
}

async fn transcribe(
    State(state): State<ApiState>,
    Query(query): Query<TranscribeQuery>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let content_type = match headers
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
    {
        Some(content_type) => content_type.to_string(),
        None => {
            return text_response(
                StatusCode::UNSUPPORTED_MEDIA_TYPE,
                "缺少 Content-Type 请求头",
            )
        }
    };
    let config = state.app.state::<ConfigManager>().get();
    let template_id = match resolve_template(&config, query.template.as_deref()) {
        Ok(template_id) => template_id,
        Err(error) => return text_response(StatusCode::BAD_REQUEST, error),
    };
    let lease = match task::start_external_task(&state.app) {
        Ok(lease) => lease,
        Err(error) => return text_response(StatusCode::TOO_MANY_REQUESTS, error),
    };
    let token = lease.token();
    let max_duration_seconds = config.general.max_recording_seconds;
    let audio_bytes = body.to_vec();
    let normalized = match tokio::task::spawn_blocking(move || {
        normalize_audio(&audio_bytes, &content_type, max_duration_seconds)
    })
    .await
    {
        Ok(Ok(audio)) => audio,
        Ok(Err(error)) => return text_response(StatusCode::BAD_REQUEST, error),
        Err(error) => {
            return text_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("音频转换任务失败：{}", error),
            )
        }
    };
    let audio_base64 = base64::engine::general_purpose::STANDARD.encode(normalized.flac);
    let result = task::run_silent_pipeline(&state.app, audio_base64, token, template_id).await;
    lease.finish();

    match result {
        Ok(text) => text_response(StatusCode::OK, text),
        Err(error) if error.contains("timed out") => {
            text_response(StatusCode::GATEWAY_TIMEOUT, error)
        }
        Err(error) => text_response(StatusCode::BAD_GATEWAY, error),
    }
}

fn text_response(status: StatusCode, body: impl Into<String>) -> Response {
    (
        status,
        [(header::CONTENT_TYPE, "text/plain; charset=utf-8")],
        body.into(),
    )
        .into_response()
}

fn resolve_template(config: &AppConfig, requested: Option<&str>) -> Result<String, String> {
    match requested {
        Some("raw") => Ok(String::new()),
        Some(template_id)
            if config
                .voice_templates
                .templates
                .iter()
                .any(|template| template.id == template_id) =>
        {
            Ok(template_id.to_string())
        }
        Some(template_id) => Err(format!("未知语音模板：{}", template_id)),
        None => Ok(config.general.shortcut_template.clone()),
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

    #[test]
    fn missing_template_uses_the_first_shortcut_binding() {
        let config = AppConfig::default();

        assert_eq!(resolve_template(&config, None).unwrap(), "voice-optimize");
    }

    #[test]
    fn raw_template_skips_text_optimization() {
        let config = AppConfig::default();

        assert_eq!(resolve_template(&config, Some("raw")), Ok(String::new()));
    }

    #[test]
    fn unknown_template_is_rejected() {
        let config = AppConfig::default();

        assert_eq!(
            resolve_template(&config, Some("missing")),
            Err("未知语音模板：missing".to_string())
        );
    }
}
