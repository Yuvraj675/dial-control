use crate::platform::traits::ContextDetector;
use std::process::Command;
use std::thread;
use std::time::Duration;
use tauri::{AppHandle, Emitter};

pub struct MacContext;

impl MacContext {
    pub fn new() -> Self {
        Self
    }

    fn query_frontmost_window() -> Option<(String, String, String)> {
        // Queries active application name, bundle ID, and active window title via osascript
        let script = r#"
            try
                tell application "System Events"
                    set frontApp to first application process whose frontmost is true
                    set appName to name of frontApp
                    set bundleID to bundle identifier of frontApp
                    set winTitle to ""
                    try
                        set winTitle to name of front window of frontApp
                    end try
                    return appName & "|||" & bundleID & "|||" & winTitle
                end tell
            on error
                return ""
            end try
        "#;

        if let Ok(output) = Command::new("osascript").args(["-e", script]).output() {
            let text = String::from_utf8_lossy(&output.stdout);
            let trimmed = text.trim();
            if trimmed.is_empty() {
                return None;
            }

            let parts: Vec<&str> = trimmed.split("|||").collect();
            if parts.len() >= 3 {
                return Some((
                    parts[0].trim().to_string(),
                    parts[1].trim().to_string(),
                    parts[2].trim().to_string(),
                ));
            } else if parts.len() == 2 {
                return Some((
                    parts[0].trim().to_string(),
                    parts[1].trim().to_string(),
                    String::new(),
                ));
            } else if !parts.is_empty() {
                return Some((parts[0].trim().to_string(), String::new(), String::new()));
            }
        }

        None
    }

    fn map_bundle_to_profile(app: &str, bundle_id: &str, title: &str) -> String {
        let combined = format!("{} {} {}", app, bundle_id, title).to_lowercase();

        if combined.contains("chrome")
            || combined.contains("brave")
            || combined.contains("safari")
            || combined.contains("firefox")
            || combined.contains("arc")
            || combined.contains("edge")
        {
            "browser".to_string()
        } else if combined.contains("code")
            || combined.contains("xcode")
            || combined.contains("sublime")
            || combined.contains("terminal")
            || combined.contains("iterm")
            || combined.contains("kitty")
            || combined.contains("alacritty")
        {
            "developer".to_string()
        } else if combined.contains("spotify")
            || combined.contains("music")
            || combined.contains("vlc")
            || combined.contains("quicktime")
        {
            "media".to_string()
        } else if combined.contains("figma") || combined.contains("sketch") {
            "figma".to_string()
        } else {
            "default".to_string()
        }
    }
}

impl ContextDetector for MacContext {
    fn start(&self, app: AppHandle) {
        thread::spawn(move || {
            let mut last_app = String::new();
            let mut last_title = String::new();

            loop {
                thread::sleep(Duration::from_millis(500));

                if let Some((app_name, bundle_id, title)) = Self::query_frontmost_window() {
                    if app_name != last_app || title != last_title {
                        let profile_id =
                            Self::map_bundle_to_profile(&app_name, &bundle_id, &title);

                        println!(
                            "[macOS Context] Active -> App: '{}', Bundle: '{}', Title: '{}' -> Matched Profile: '{}'",
                            app_name, bundle_id, title, profile_id
                        );

                        last_app = app_name.clone();
                        last_title = title.clone();

                        let _ = app.emit(
                            "context:change",
                            serde_json::json!({
                                "profile_id": profile_id,
                                "app": app_name,
                                "wm_class": bundle_id,
                                "title": title,
                            }),
                        );
                    }
                }
            }
        });
    }
}
