//! 模型 Token 用量记录。
//!
//! 每次成功的模型调用追加一条 JSONL 记录（usage.jsonl，与应用数据目录同级），
//! 按行追加避免全量重写；记录永久保留，不上传。服务商未返回用量时按 0 记，
//! 保证调用次数统计不受影响。

use serde::{Deserialize, Serialize};
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use tauri::{AppHandle, Emitter};

const USAGE_FILE: &str = "usage.jsonl";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageRecord {
    /// 毫秒时间戳
    pub ts: u64,
    /// transcribe / extract / optimize / learn
    pub scene: String,
    /// 解析后的模型名（发给服务商的 model 字符串）
    pub model: String,
    /// 服务商显示名
    pub provider: String,
    pub input_tokens: u64,
    pub output_tokens: u64,
}

struct UsageState {
    path: PathBuf,
    records: Vec<UsageRecord>,
    app: Option<AppHandle>,
}

static STATE: OnceLock<Mutex<UsageState>> = OnceLock::new();

fn state() -> &'static Mutex<UsageState> {
    STATE.get_or_init(|| {
        Mutex::new(UsageState {
            path: PathBuf::new(),
            records: Vec::new(),
            app: None,
        })
    })
}

/// 应用启动时调用：加载数据目录下的历史用量记录。
pub fn init(data_dir: &std::path::Path, app: AppHandle) {
    let path = data_dir.join(USAGE_FILE);
    let mut st = state().lock().unwrap_or_else(|e| e.into_inner());
    st.path = path.clone();
    st.app = Some(app);
    if let Ok(content) = std::fs::read_to_string(&path) {
        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            match serde_json::from_str::<UsageRecord>(line) {
                Ok(record) => st.records.push(record),
                Err(e) => eprintln!("[Usage] 跳过无法解析的记录行: {}", e),
            }
        }
    }
}

/// 记录一次成功的模型调用。写入失败只打日志，不影响主流程。
pub fn record(scene: &str, model: &str, provider: &str, input_tokens: u64, output_tokens: u64) {
    let entry = UsageRecord {
        ts: now_millis(),
        scene: scene.to_string(),
        model: model.to_string(),
        provider: provider.to_string(),
        input_tokens,
        output_tokens,
    };
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
            Err(e) => eprintln!("[Usage] 写入用量记录失败: {}", e),
        }
    }

    st.records.push(entry);
    if let Some(app) = &st.app {
        let _ = app.emit("usage-updated", ());
    }
}

pub fn get_records() -> Vec<UsageRecord> {
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
    std::fs::write(&st.path, "").map_err(|e| format!("清空用量记录失败: {}", e))?;
    st.records.clear();
    if let Some(app) = &st.app {
        let _ = app.emit("usage-updated", ());
    }
    Ok(())
}

fn now_millis() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn usage_record_serializes_camel_case() {
        let record = UsageRecord {
            ts: 1700000000000,
            scene: "transcribe".to_string(),
            model: "gemini-3.8-flash".to_string(),
            provider: "Google Gemini".to_string(),
            input_tokens: 120,
            output_tokens: 45,
        };
        let json = serde_json::to_string(&record).unwrap();
        assert!(json.contains("\"inputTokens\":120"));
        assert!(json.contains("\"outputTokens\":45"));
        let back: UsageRecord = serde_json::from_str(&json).unwrap();
        assert_eq!(back.scene, "transcribe");
        assert_eq!(back.ts, 1700000000000);
    }
}
