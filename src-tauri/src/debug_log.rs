use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::sync::OnceLock;
use std::time::Instant;

static LOG_PATH: OnceLock<PathBuf> = OnceLock::new();
static START: OnceLock<Instant> = OnceLock::new();

pub fn init(data_dir: &std::path::Path) {
    let path = data_dir.join("screenshot-debug.log");
    let _ = std::fs::write(&path, "");
    let _ = LOG_PATH.set(path);
    let _ = START.set(Instant::now());
}

pub fn log(msg: &str) {
    let elapsed = START.get().map(|s| s.elapsed().as_secs_f64()).unwrap_or(0.0);
    let line = format!("[{:.3}s] {}\n", elapsed, msg);
    eprint!("{}", line);
    if let Some(path) = LOG_PATH.get() {
        if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(path) {
            let _ = f.write_all(line.as_bytes());
        }
    }
}

#[tauri::command]
pub fn js_debug_log(msg: String) {
    log(&format!("[JS] {}", msg));
}
