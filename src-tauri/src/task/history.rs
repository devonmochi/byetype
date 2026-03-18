use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use base64::Engine as _;

const MAX_RECORDS: usize = 3;

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
}

pub struct HistoryManager {
    history_dir: PathBuf,
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
            history_dir,
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
            self.records = serde_json::from_str(&content).unwrap_or_default();
        }
        // Validate audio paths
        for record in &mut self.records {
            if let Some(ref path) = record.audio_path {
                if !Path::new(path).exists() {
                    record.audio_path = None;
                }
            }
        }
        // Clean orphan audio files
        if let Ok(entries) = std::fs::read_dir(&self.audio_dir) {
            let referenced: std::collections::HashSet<String> = self
                .records
                .iter()
                .filter_map(|r| r.audio_path.clone())
                .collect();
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
        });
        while self.records.len() > MAX_RECORDS {
            let oldest = self.records.remove(0);
            if let Some(ref path) = oldest.audio_path {
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
        if let Some(record) = self.records.iter_mut().find(|r| r.id == id) {
            if let Some(t) = transcribe_text {
                record.transcribe_text = Some(t);
            }
            if let Some(o) = optimize_text {
                record.optimize_text = Some(o);
            }
            record.status = status.to_string();
            record.error_message = error_message;
            self.persist()?;
        }
        Ok(())
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

    pub fn clear(&mut self) -> Result<(), String> {
        if self.history_dir.exists() {
            let _ = std::fs::remove_dir_all(&self.history_dir);
        }
        self.records.clear();
        std::fs::create_dir_all(&self.audio_dir)
            .map_err(|e| format!("Failed to recreate history dir: {}", e))?;
        self.persist()
    }

    fn persist(&self) -> Result<(), String> {
        let data = serde_json::to_string_pretty(&self.records)
            .map_err(|e| format!("Failed to serialize history: {}", e))?;
        std::fs::write(&self.json_path, data)
            .map_err(|e| format!("Failed to write history: {}", e))
    }
}

/// ISO-8601 timestamp without chrono dependency
fn now_iso() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let dur = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let secs = dur.as_secs();
    let days = secs / 86400;
    let time_secs = secs % 86400;
    let hours = time_secs / 3600;
    let mins = (time_secs % 3600) / 60;
    let s = time_secs % 60;
    let (y, m, d) = days_to_ymd(days as i64);
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.000Z",
        y, m, d, hours, mins, s
    )
}

fn days_to_ymd(days: i64) -> (i64, u32, u32) {
    let z = days + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u32;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}
