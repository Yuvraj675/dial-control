use crate::platform::traits::ContextDetector;
use futures_lite::StreamExt;
use std::time::Duration;
use tauri::{AppHandle, Emitter};

pub use crate::platform::common::{match_profile, ContextEvent};

#[derive(Debug, serde::Deserialize)]
struct FocusedWindowJson {
    title: Option<String>,
    wm_class: Option<String>,
    #[serde(rename = "wm_class_instance")]
    _wm_class_instance: Option<String>,
    pid: Option<u32>,
}

pub fn ensure_bundled_extension_installed() {
    if let Ok(home) = std::env::var("HOME") {
        let ext_dir = std::path::PathBuf::from(home)
            .join(".local/share/gnome-shell/extensions/focused-window-dbus@flexagoon.com");
        let _ = std::fs::create_dir_all(&ext_dir);
        let ext_js_path = ext_dir.join("extension.js");
        let meta_json_path = ext_dir.join("metadata.json");

        const EXT_JS: &str = include_str!("../../../../resources/gnome-extension/focused-window-dbus@flexagoon.com/extension.js");
        const META_JSON: &str = include_str!("../../../../resources/gnome-extension/focused-window-dbus@flexagoon.com/metadata.json");

        let _ = std::fs::write(&ext_js_path, EXT_JS);
        let _ = std::fs::write(&meta_json_path, META_JSON);
    }
}

pub struct LinuxContext;

impl LinuxContext {
    pub fn new() -> Self {
        Self
    }
}


// ── Compositor Fallbacks ───────────────────────────────────────────────────

fn find_sway_focused_node(node: &serde_json::Value) -> Option<(String, String)> {
    if node.get("focused").and_then(|v| v.as_bool()) == Some(true) {
        let app = node
            .get("app_id")
            .and_then(|v| v.as_str())
            .or_else(|| {
                node.get("window_properties")
                    .and_then(|p| p.get("class"))
                    .and_then(|v| v.as_str())
            })
            .unwrap_or("")
            .to_string();
        let name = node
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        return Some((app, name));
    }
    if let Some(nodes) = node.get("nodes").and_then(|v| v.as_array()) {
        for n in nodes {
            if let Some(res) = find_sway_focused_node(n) {
                return Some(res);
            }
        }
    }
    if let Some(floating) = node.get("floating_nodes").and_then(|v| v.as_array()) {
        for n in floating {
            if let Some(res) = find_sway_focused_node(n) {
                return Some(res);
            }
        }
    }
    None
}

fn get_active_window_fallback() -> Option<(String, String)> {
    // 1. Hyprland
    if std::env::var("HYPRLAND_INSTANCE_SIGNATURE").is_ok() {
        if let Ok(out) = std::process::Command::new("hyprctl")
            .args(["activewindow", "-j"])
            .output()
        {
            if let Ok(val) = serde_json::from_slice::<serde_json::Value>(&out.stdout) {
                let class = val
                    .get("class")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let title = val
                    .get("title")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                if !class.is_empty() || !title.is_empty() {
                    return Some((class, title));
                }
            }
        }
    }

    // 2. Sway
    if std::env::var("SWAYSOCK").is_ok() {
        if let Ok(out) = std::process::Command::new("swaymsg")
            .args(["-t", "get_tree"])
            .output()
        {
            if let Ok(val) = serde_json::from_slice::<serde_json::Value>(&out.stdout) {
                if let Some(res) = find_sway_focused_node(&val) {
                    return Some(res);
                }
            }
        }
    }

    // 3. KDE Plasma
    if let Ok(out) = std::process::Command::new("kdotool")
        .arg("getactivewindow")
        .output()
    {
        if out.status.success() {
            let win_id = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if !win_id.is_empty() {
                if let Ok(title_out) = std::process::Command::new("kdotool")
                    .args(["getwindowname", &win_id])
                    .output()
                {
                    let title = String::from_utf8_lossy(&title_out.stdout)
                        .trim()
                        .to_string();
                    return Some(("kde_app".to_string(), title));
                }
            }
        }
    }

    // 4. X11 / XWayland via xdotool
    if std::env::var("DISPLAY").is_ok() {
        if let Ok(out) = std::process::Command::new("xdotool")
            .args(["getactivewindow", "getwindowname"])
            .output()
        {
            if out.status.success() {
                let title = String::from_utf8_lossy(&out.stdout).trim().to_string();
                if !title.is_empty() {
                    return Some(("x11_window".to_string(), title));
                }
            }
        }
    }

    None
}

impl ContextDetector for LinuxContext {
    fn start(&self, app: AppHandle) {
        std::thread::spawn(move || {
            let rt = match tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
            {
                Ok(r) => r,
                Err(e) => {
                    eprintln!("[!] Failed to build context tokio runtime: {:?}", e);
                    return;
                }
            };

            rt.block_on(async move {
                let mut last_profile = String::new();
                let mut last_title = String::new();
                let my_pid = std::process::id();

                loop {
                    // Attempt connection to session D-Bus for GNOME Shell FocusedWindow extension
                    let connection = match zbus::Connection::session().await {
                        Ok(c) => Some(c),
                        Err(e) => {
                            eprintln!("[Linux Context] Session D-Bus connection: {:?}", e);
                            None
                        }
                    };

                    if let Some(ref conn) = connection {
                        // Check if FocusedWindow extension is active
                        let proxy = zbus::Proxy::new(
                            conn,
                            "org.gnome.Shell",
                            "/org/gnome/shell/extensions/FocusedWindow",
                            "org.gnome.shell.extensions.FocusedWindow",
                        )
                        .await;

                        let mut has_extension = if let Ok(ref p) = proxy {
                            p.call_method("Get", &()).await.is_ok()
                        } else {
                            false
                        };

                        // Ensure our bundled extension (with MoveWindow support) is installed on GNOME Wayland
                        ensure_bundled_extension_installed();

                        if !has_extension {
                            println!("[Linux Context] Enabling FocusedWindow extension via GNOME Shell D-Bus...");
                            if let Ok(shell_proxy) = zbus::Proxy::new(
                                conn,
                                "org.gnome.Shell",
                                "/org/gnome/Shell",
                                "org.gnome.Shell.Extensions",
                            )
                            .await
                            {
                                let _ = shell_proxy
                                    .call_method(
                                        "EnableExtension",
                                        &("focused-window-dbus@flexagoon.com",),
                                    )
                                    .await;
                            }

                            tokio::time::sleep(Duration::from_millis(500)).await;

                            if let Ok(ref p) = proxy {
                                has_extension = p.call_method("Get", &()).await.is_ok();
                            }
                        }

                        if has_extension {
                            println!("[Linux Context] Active listener registered on GNOME FocusedWindow D-Bus service.");
                            let rule = match zbus::MatchRule::builder()
                                .msg_type(zbus::message::Type::Signal)
                                .sender("org.gnome.Shell")
                            {
                                Ok(b) => match b.interface("org.gnome.shell.extensions.FocusedWindow") {
                                    Ok(b) => match b.member("FocusChanged") {
                                        Ok(b) => Some(b.build()),
                                        Err(_) => None,
                                    },
                                    Err(_) => None,
                                },
                                Err(_) => None,
                            };

                            let mut stream = if let Some(r) = rule {
                                zbus::MessageStream::for_match_rule(r, conn, None)
                                    .await
                                    .ok()
                            } else {
                                None
                            };

                            let win_proxy = proxy.unwrap();

                            // Primary monitoring loop for GNOME
                            loop {
                                let window_json_str: Option<String> = tokio::select! {
                                    Some(Ok(msg)) = async {
                                        if let Some(ref mut s) = stream {
                                            s.next().await
                                        } else {
                                            tokio::time::sleep(Duration::from_secs(3600)).await;
                                            None
                                        }
                                    } => {
                                        msg.body().deserialize::<(String,)>().map(|(s,)| s)
                                            .or_else(|_| msg.body().deserialize::<String>())
                                            .ok()
                                    }
                                    _ = tokio::time::sleep(Duration::from_millis(350)) => {
                                        if let Ok(msg) = win_proxy.call_method("Get", &()).await {
                                            msg.body().deserialize::<(String,)>().map(|(s,)| s)
                                                .or_else(|_| msg.body().deserialize::<String>())
                                                .ok()
                                        } else {
                                            None
                                        }
                                    }
                                };

                                if let Some(raw_json) = window_json_str {
                                    if let Ok(win) = serde_json::from_str::<FocusedWindowJson>(&raw_json) {
                                        let pid = win.pid.unwrap_or(0);
                                        let wm_class = win.wm_class.unwrap_or_default();
                                        let title = win.title.unwrap_or_default();

                                        // Ignore self / HUD window
                                        if pid == my_pid
                                            || wm_class == "dial-control"
                                            || title.contains("Dial Control")
                                        {
                                            continue;
                                        }

                                        let profile_id = match_profile(&wm_class, &wm_class, &title);

                                        if profile_id != last_profile || title != last_title {
                                            last_profile = profile_id.to_string();
                                            last_title = title.clone();

                                            println!(
                                                "[Linux Context] Focus -> App: '{}' | Title: '{}' | Profile: {}",
                                                wm_class, title, profile_id
                                            );

                                            let event = ContextEvent {
                                                profile_id: profile_id.to_string(),
                                                app: wm_class.clone(),
                                                title: title.clone(),
                                                wm_class: wm_class.clone(),
                                            };
                                            let _ = app.emit("context:change", event);
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // Fallback polling for other compositors (Hyprland, Sway, KDE, X11)
                    if let Some((app_name, title)) = get_active_window_fallback() {
                        let profile_id = match_profile(&app_name, &app_name, &title);
                        if profile_id != last_profile || title != last_title {
                            last_profile = profile_id.to_string();
                            last_title = title.clone();

                            println!(
                                "[Linux Context Fallback] Focus -> App: '{}' | Title: '{}' | Profile: {}",
                                app_name, title, profile_id
                            );

                            let event = ContextEvent {
                                profile_id: profile_id.to_string(),
                                app: app_name.clone(),
                                title: title.clone(),
                                wm_class: app_name.clone(),
                            };
                            let _ = app.emit("context:change", event);
                        }
                    }

                    tokio::time::sleep(Duration::from_millis(500)).await;
                }
            });
        });
    }
}
