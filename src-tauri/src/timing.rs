//! 录音处理耗时记录。
//!
//! 每次成功的语音录音（文本成功粘贴）追加一条 JSONL 记录（timing.jsonl，
//! 与 usage.jsonl 同目录），按行追加避免全量重写；记录永久保留，不上传。
//! 分阶段计时：音频转写、文本优化各自含自动重试耗时，剩余零散时间归
//! other，total 为从停止录音到粘贴完成的总耗时。

use serde::{Deserialize, Serialize};
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use tauri::{AppHandle, Emitter};

const TIMING_FILE: &str = "timing.jsonl";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimingRecord {
    /// 毫秒时间戳
    pub ts: u64,
    /// 音频转写阶段耗时（含自动重试），毫秒
    pub transcribe_ms: u64,
    /// 文本优化阶段耗时（含自动重试），未启用优化时为 0
    pub optimize_ms: u64,
    /// 其他零散耗时（网络建连、粘贴等）
    pub other_ms: u64,
    /// 从停止录音到粘贴完成的总耗时
    pub total_ms: u64,
    /// 转写模型名
    pub transcribe_model: String,
    /// 转写服务商显示名
    pub transcribe_provider: String,
    /// 优化模型名，未启用优化时为 None
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub optimize_model: Option<String>,
    /// 优化服务商显示名，未启用优化时为 None
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub optimize_provider: Option<String>,
}

struct TimingState {
    path: PathBuf,
    records: Vec<TimingRecord>,
    app: Option<AppHandle>,
}

static STATE: OnceLock<Mutex<TimingState>> = OnceLock::new();

fn state() -> &'static Mutex<TimingState> {
    STATE.get_or_init(|| {
        Mutex::new(TimingState {
            path: PathBuf::new(),
            records: Vec::new(),
            app: None,
        })
    })
}

/// 应用启动时调用：加载数据目录下的历史耗时记录。
pub fn init(data_dir: &std::path::Path, app: AppHandle) {
    let path = data_dir.join(TIMING_FILE);
    let mut st = state().lock().unwrap_or_else(|e| e.into_inner());
    st.path = path.clone();
    st.app = Some(app);
    if let Ok(content) = std::fs::read_to_string(&path) {
        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            match serde_json::from_str::<TimingRecord>(line) {
                Ok(record) => st.records.push(record),
                Err(e) => eprintln!("[Timing] 跳过无法解析的记录行: {}", e),
            }
        }
    }
}

/// 记录一次成功的录音处理。写入失败只打日志，不影响主流程。
pub fn record(entry: TimingRecord) {
    let mut st = state().lock().unwrap_or_else(|e| e.into_inner());

    if !st.path.as_os_str().is_empty() {
        match OpenOptions::new()
            .create(true)
            .append(true)
            .open(&st.path)
            .and_then(|mut file| {
                let line = serde_json::to_string(&entry).unwrap_or_default();
                writeln!(file, "{}", line)
            }) {
            Ok(()) => {}
            Err(e) => eprintln!("[Timing] 写入耗时记录失败: {}", e),
        }
    }

    st.records.push(entry);
    if let Some(app) = &st.app {
        let _ = app.emit("timing-updated", ());
    }
}

pub fn get_records() -> Vec<TimingRecord> {
    state()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .records
        .clone()
}

pub fn clear() -> Result<(), String> {
    let mut st = state().lock().unwrap_or_else(|e| e.into_inner());
    if st.path.as_os_str().is_empty() {
        return Ok(());
    }
    std::fs::write(&st.path, "").map_err(|e| format!("清空耗时记录失败: {}", e))?;
    st.records.clear();
    if let Some(app) = &st.app {
        let _ = app.emit("timing-updated", ());
    }
    Ok(())
}

pub fn now_millis() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timing_record_serializes_camel_case() {
        let record = TimingRecord {
            ts: 1700000000000,
            transcribe_ms: 3200,
            optimize_ms: 1850,
            other_ms: 370,
            total_ms: 5420,
            transcribe_model: "gemini-2.5-flash".to_string(),
            transcribe_provider: "Google Gemini".to_string(),
            optimize_model: Some("deepseek-v3".to_string()),
            optimize_provider: Some("DeepSeek".to_string()),
        };
        let json = serde_json::to_string(&record).unwrap();
        assert!(json.contains("\"transcribeMs\":3200"));
        assert!(json.contains("\"totalMs\":5420"));
        let back: TimingRecord = serde_json::from_str(&json).unwrap();
        assert_eq!(back.transcribe_model, "gemini-2.5-flash");
        assert_eq!(back.optimize_model.as_deref(), Some("deepseek-v3"));
        assert_eq!(back.optimize_provider.as_deref(), Some("DeepSeek"));
    }

    #[test]
    fn timing_record_without_optimize_omits_field() {
        let record = TimingRecord {
            ts: 1700000000000,
            transcribe_ms: 3200,
            optimize_ms: 0,
            other_ms: 370,
            total_ms: 3570,
            transcribe_model: "gemini-2.5-flash".to_string(),
            transcribe_provider: "Google Gemini".to_string(),
            optimize_model: None,
            optimize_provider: None,
        };
        let json = serde_json::to_string(&record).unwrap();
        assert!(!json.contains("optimizeModel"));
        let back: TimingRecord = serde_json::from_str(&json).unwrap();
        assert_eq!(back.optimize_model, None);
        assert_eq!(back.optimize_provider, None);
        assert_eq!(back.optimize_ms, 0);
    }
}
