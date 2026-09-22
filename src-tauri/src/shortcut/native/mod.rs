//! Native backend abstraction for the standalone physical right-Alt key.
//!
//! Windows prefers Raw Input and falls back to a low-level keyboard hook.
//! Other platforms have no native backend; a config value of `AltRight` there is
//! simply skipped at registration time.

use std::sync::Arc;

pub type EventSink = Arc<dyn Fn(bool) + Send + Sync>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendKind {
    RawInput,
    LowLevelHook,
}

impl BackendKind {
    pub fn as_str(self) -> &'static str {
        match self {
            BackendKind::RawInput => "raw_input",
            BackendKind::LowLevelHook => "low_level_hook",
        }
    }
}

/// A running native AltRight backend. Dropping or stopping it releases the
/// OS-level registration and joins the listener thread.
pub struct AltRightBackend {
    kind: BackendKind,
    stop: Option<Box<dyn FnOnce() + Send>>,
}

impl AltRightBackend {
    pub fn kind(&self) -> BackendKind {
        self.kind
    }

    pub fn stop(&mut self) {
        if let Some(stop) = self.stop.take() {
            stop();
        }
    }
}

impl Drop for AltRightBackend {
    fn drop(&mut self) {
        self.stop();
    }
}

#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "windows")]
pub fn start_alt_right(sink: EventSink) -> Result<AltRightBackend, String> {
    windows::start(sink)
}

#[cfg(not(target_os = "windows"))]
pub fn start_alt_right(_sink: EventSink) -> Result<AltRightBackend, String> {
    Err("ALT_RIGHT_BACKEND_FAILED: 当前平台不支持原生右 Alt 监听".to_string())
}

#[allow(dead_code)]
pub(crate) fn make_backend(
    kind: BackendKind,
    stop: Box<dyn FnOnce() + Send>,
) -> AltRightBackend {
    AltRightBackend {
        kind,
        stop: Some(stop),
    }
}
