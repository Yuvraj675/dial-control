//! Tiny timestamped diagnostic log for field debugging on machines without a console
//! (release Windows builds hide stdout). Written to `<temp>/dial-control/dial-control.log`.

use std::io::Write;
use std::sync::OnceLock;
use std::time::Instant;

static START: OnceLock<Instant> = OnceLock::new();

fn log_path() -> std::path::PathBuf {
    std::env::temp_dir().join("dial-control").join("dial-control.log")
}

/// Truncates an oversized log from a previous run. Call once at startup.
pub fn init() {
    START.get_or_init(Instant::now);
    let path = log_path();
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if std::fs::metadata(&path).map(|m| m.len() > 512 * 1024).unwrap_or(false) {
        let _ = std::fs::remove_file(&path);
    }
    log("---- Dial Control started ----");
}

/// Appends `[+<ms since start>] msg`; never fails or blocks the caller meaningfully.
pub fn log(msg: &str) {
    let ms = START.get_or_init(Instant::now).elapsed().as_millis();
    let line = format!("[+{ms:>8}ms] {msg}\n");
    print!("{line}");
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(log_path()) {
        let _ = f.write_all(line.as_bytes());
    }
}
