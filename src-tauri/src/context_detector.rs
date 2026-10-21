use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;
use tauri::{AppHandle, Emitter};

pub fn start_context_detector(app: AppHandle) {
    thread::spawn(move || {
        loop {
            // Check if local prototype script exists in temp/tools or tools for development fallback
            let script_path = if std::path::Path::new("temp/tools/context_detector.py").exists() {
                "temp/tools/context_detector.py"
            } else if std::path::Path::new("tools/context_detector.py").exists() {
                "tools/context_detector.py"
            } else {
                ""
            };

            if script_path.is_empty() {
                // Graceful fallback when running standalone without legacy python prototypes
                thread::sleep(Duration::from_secs(5));
                continue;
            }

            let mut child = match Command::new("python3")
                .args([script_path, "--stream"])
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .spawn()
            {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("[!] Failed to spawn context detector: {:?}. Retrying in 5s...", e);
                    thread::sleep(Duration::from_secs(5));
                    continue;
                }
            };

            if let Some(stdout) = child.stdout.take() {
                let reader = BufReader::new(stdout);
                for line in reader.lines() {
                    if let Ok(line_str) = line {
                        let trimmed = line_str.trim();
                        if trimmed.is_empty() {
                            continue;
                        }

                        if let Ok(json) = serde_json::from_str::<serde_json::Value>(trimmed) {
                            println!("[Context] Active foreground: {:?}", json);
                            let _ = app.emit("context:change", json);
                        }
                    }
                }
            }

            let _ = child.wait();
            thread::sleep(Duration::from_secs(2));
        }
    });
}
