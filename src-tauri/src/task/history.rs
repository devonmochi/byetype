use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use base64::Engine as _;

const MAX_RECORDS: usize = 3;

fn default_record_type() -> String {
    "voice".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryRecord {
    pub id: u64,
    pub created_at: String,
    pub audio_path: Option<String>,
    pub transcribe_text: Option<String>,
    pub optimize_text: Option<String>,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
    #[serde(default = "default_record_type")]
    pub record_type: String,
    #[serde(default)]
    pub screenshot_path: Option<String>,
    #[serde(default)]
    pub extract_text: Option<String>,
}

pub struct HistoryManager {
    audio_dir: PathBuf,
    json_path: PathBuf,
    records: Vec<HistoryRecord>,
    last_id: u64,
}

impl HistoryManager {
    pub fn new(data_dir: &Path) -> Self {
        let history_dir = data_dir.join("history");
        let audio_dir = history_dir.join("audio");
        let json_path = history_dir.join("history.json");
        Self {
            audio_dir,
            json_path,
            records: Vec::new(),
            last_id: 0,
        }
    }

    pub fn init(&mut self) -> Result<(), String> {
        std::fs::create_dir_all(&self.audio_dir)
            .map_err(|e| format!("Failed to create history dir: {}", e))?;
        if self.json_path.exists() {
            let content = std::fs::read_to_string(&self.json_path).unwrap_or_default();
            match serde_json::from_str::<Vec<HistoryRecord>>(&content) {
                Ok(v) => {
                    self.records = v;
                }
                Err(e) => {
                    eprintln!(
                        "[History] history.json 解析失败，保留空记录但不删除原文件: {}",
                        e
                    );
                    let corrupt_path = format!("{}.corrupt", self.json_path.display());
                    let _ = std::fs::rename(&self.json_path, &corrupt_path);
                    self.records = Vec::new();
                }
            }
        }
        // Validate audio and screenshot paths
        for record in &mut self.records {
            if let Some(ref path) = record.audio_path {
                if !Path::new(path).exists() {
                    record.audio_path = None;
                }
            }
            if let Some(ref path) = record.screenshot_path {
                if !Path::new(path).exists() {
                    record.screenshot_path = None;
                }
            }
        }
        // Clean orphan audio/screenshot files
        if let Ok(entries) = std::fs::read_dir(&self.audio_dir) {
            let mut referenced: std::collections::HashSet<String> = self
                .records
                .iter()
                .filter_map(|r| r.audio_path.clone())
                .collect();
            referenced.extend(self.records.iter().filter_map(|r| r.screenshot_path.clone()));
            for entry in entries.flatten() {
                let path = entry.path().to_string_lossy().to_string();
                if !referenced.contains(&path) {
                    let _ = std::fs::remove_file(entry.path());
                }
            }
        }
        if let Some(max_id) = self.records.iter().map(|r| r.id).max() {
            self.last_id = max_id;
        }
        self.persist()
    }

    fn next_id(&mut self) -> u64 {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        self.last_id = if now > self.last_id {
            now
        } else {
            self.last_id + 1
        };
        self.last_id
    }

    pub fn add_record(
        &mut self,
        audio_base64: Option<&str>,
        transcribe_text: Option<String>,
        optimize_text: Option<String>,
        status: &str,
        error_message: Option<String>,
    ) -> Result<(), String> {
        let id = self.next_id();
        let mut audio_path: Option<String> = None;
        if let Some(b64) = audio_base64 {
            let dest = self.audio_dir.join(format!("{}.wav", id));
            let bytes = base64::engine::general_purpose::STANDARD
                .decode(b64)
                .map_err(|e| format!("Failed to decode audio: {}", e))?;
            std::fs::write(&dest, &bytes)
                .map_err(|e| format!("Failed to write audio: {}", e))?;
            audio_path = Some(dest.to_string_lossy().to_string());
        }
        self.records.push(HistoryRecord {
            id,
            created_at: now_iso(),
            audio_path,
            transcribe_text,
            optimize_text,
            status: status.to_string(),
            error_message,
            record_type: "voice".to_string(),
            screenshot_path: None,
            extract_text: None,
        });
        while self.records.len() > MAX_RECORDS {
            let oldest = self.records.remove(0);
            if let Some(ref path) = oldest.audio_path {
                let _ = std::fs::remove_file(path);
            }
            if let Some(ref path) = oldest.screenshot_path {
                let _ = std::fs::remove_file(path);
            }
        }
        self.persist()
    }

    pub fn add_extract_record(
        &mut self,
        screenshot_base64: Option<&str>,
        extract_text: Option<String>,
        status: &str,
        error_message: Option<String>,
    ) -> Result<(), String> {
        let id = self.next_id();
        let mut screenshot_path: Option<String> = None;
        if let Some(b64) = screenshot_base64 {
            let dest = self.audio_dir.join(format!("{}.png", id));
            let bytes = base64::engine::general_purpose::STANDARD
                .decode(b64)
                .map_err(|e| format!("Failed to decode screenshot: {}", e))?;
            std::fs::write(&dest, &bytes)
                .map_err(|e| format!("Failed to write screenshot: {}", e))?;
            screenshot_path = Some(dest.to_string_lossy().to_string());
        }
        self.records.push(HistoryRecord {
            id,
            created_at: now_iso(),
            audio_path: None,
            transcribe_text: None,
            optimize_text: None,
            status: status.to_string(),
            error_message,
            record_type: "extract".to_string(),
            screenshot_path,
            extract_text,
        });
        while self.records.len() > MAX_RECORDS {
            let oldest = self.records.remove(0);
            if let Some(ref path) = oldest.audio_path {
                let _ = std::fs::remove_file(path);
            }
            if let Some(ref path) = oldest.screenshot_path {
                let _ = std::fs::remove_file(path);
            }
        }
        self.persist()
    }

    pub fn update_record(
        &mut self,
        id: u64,
        transcribe_text: Option<String>,
        optimize_text: Option<String>,
        status: &str,
        error_message: Option<String>,
    ) -> Result<(), String> {
        let record = self
            .records
            .iter_mut()
            .find(|r| r.id == id)
            .ok_or_else(|| format!("record {} not found", id))?;
        if let Some(t) = transcribe_text {
            record.transcribe_text = Some(t);
        }
        if let Some(o) = optimize_text {
            record.optimize_text = Some(o);
        }
        record.status = status.to_string();
        record.error_message = error_message;
        self.persist()
    }

    pub fn get_records(&self) -> &[HistoryRecord] {
        &self.records
    }

    pub fn get_audio_base64(&self, id: u64) -> Option<String> {
        let record = self.records.iter().find(|r| r.id == id)?;
        let path = record.audio_path.as_ref()?;
        let bytes = std::fs::read(path).ok()?;
        Some(base64::engine::general_purpose::STANDARD.encode(&bytes))
    }

    fn persist(&self) -> Result<(), String> {
        let data = serde_json::to_string_pretty(&self.records)
            .map_err(|e| format!("Failed to serialize history: {}", e))?;
        let tmp = self.json_path.with_extension("json.tmp");
        std::fs::write(&tmp, &data).map_err(|e| format!("Failed to write history: {}", e))?;
        std::fs::rename(&tmp, &self.json_path)
            .map_err(|e| format!("Failed to rename history: {}", e))
    }
}

/// ISO-8601 毫秒时间戳，形如 2026-09-11T10:00:00.000Z
fn now_iso() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}
