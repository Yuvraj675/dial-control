pub mod platform;

use platform::{Platform, PlatformProvider};
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicUsize, Ordering};
use std::sync::Arc;
use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::TrayIconBuilder,
    AppHandle, Emitter, Manager,
};

static LAST_HUD_SHOWN_TIME: AtomicUsize = AtomicUsize::new(0);

pub struct AppState {
    pub platform: Arc<Platform>,
    pub is_hud_open: Arc<AtomicBool>,
    /// The user's listening volume captured when the HUD opened; restored on close.
    /// -1 = no lock (volume mode: changes are intentional and kept).
    pub locked_vol: Arc<AtomicI32>,
    pub is_recording_shortcut: Arc<AtomicBool>,
    pub is_scroll_mode: Arc<AtomicBool>,
}

/// Captures the user's volume as the restore point unless one is already held.
/// Must never overwrite an existing lock: by the time the frontend calls
/// `set_mode("menu")` the backend may already have re-centred the volume for headroom
/// (Windows) or BlueZ may have clamped it (Linux), and re-reading would lose the
/// user's real volume on close.
pub(crate) fn capture_volume_lock(state: &AppState) -> i32 {
    let locked = state.locked_vol.load(Ordering::SeqCst);
    if locked >= 0 {
        return locked;
    }
    let curr_vol = state.platform.audio().get_volume();
    state.locked_vol.store(curr_vol, Ordering::SeqCst);
    curr_vol
}

/// On macOS hiding a window does not hand focus back: our app stays active and
/// synthesized hotkeys land in Dial Control itself. Reactivate the app the user was in.
fn return_focus_to_previous_app(app: &AppHandle) {
    #[cfg(target_os = "macos")]
    {
        let hud_focused = app
            .get_webview_window("hud")
            .and_then(|w| w.is_focused().ok())
            .unwrap_or(false);
        if hud_focused {
            let _ = app.run_on_main_thread(crate::platform::macos::context::reactivate_previous_app);
        }
    }
    #[cfg(not(target_os = "macos"))]
    let _ = app;
}

// Brightness commands are `async` so Tauri runs them off the main thread: on Windows a
// WMI read spawns PowerShell (hundreds of ms) and would otherwise freeze the HUD.
#[tauri::command(async)]
fn get_system_brightness(state: tauri::State<AppState>) -> i32 {
    state.platform.brightness().get_brightness()
}

#[tauri::command(async)]
fn set_system_brightness(state: tauri::State<AppState>, pct: i32) -> i32 {
    state.platform.brightness().set_brightness(pct)
}

#[tauri::command(async)]
fn step_brightness(state: tauri::State<AppState>, direction: i32) {
    state.platform.brightness().step_brightness(direction);
}

#[tauri::command]
fn get_system_volume(state: tauri::State<AppState>) -> i32 {
    state.platform.audio().get_volume()
}

#[tauri::command]
fn set_system_volume(state: tauri::State<AppState>, pct: i32) -> i32 {
    state.platform.audio().set_volume(pct)
}

/// Returns the volume the gauge should display (the user's real volume), or -1.
#[tauri::command]
fn set_mode(app: AppHandle, state: tauri::State<AppState>, mode: String) -> i32 {
    if mode == "volume" {
        state.is_scroll_mode.store(false, Ordering::SeqCst);
        // Release the lock first so dial listeners treat the restore below as a natural change.
        let user_vol = state.locked_vol.swap(-1, Ordering::SeqCst);
        if user_vol >= 0 && state.platform.audio().get_volume() != user_vol {
            // The backend may have re-centred the volume for rotation headroom; volume
            // mode must start from the user's real level, not the headroom position.
            state.platform.audio().set_volume(user_vol);
        }
        println!("[HUD] Switched to volume mode (lock released for natural hardware volume control)");
        user_vol
    } else if mode == "scroll" {
        let user_vol = capture_volume_lock(&state);
        state.is_scroll_mode.store(true, Ordering::SeqCst);
        state.is_hud_open.store(true, Ordering::SeqCst);
        println!("[HUD] Switched to SCROLL MODE (volume locked at {}%)", user_vol);
        let _ = app.emit("scroll_mode:active", true);
        user_vol
    } else {
        state.is_scroll_mode.store(false, Ordering::SeqCst);
        let user_vol = capture_volume_lock(&state);
        println!("[HUD] Switched to mode: {} (volume locked at {}% for navigation)", mode, user_vol);
        user_vol
    }
}

pub fn send_os_notification(title: &str, body: &str) {
    #[cfg(target_os = "linux")]
    {
        let _ = std::process::Command::new("notify-send")
            .args([
                "-a", "Dial Control",
                "-h", "string:synchronous:dial",
                title,
                body,
            ])
            .spawn();
    }

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        let script = format!(
            r#"[Windows.UI.Notifications.ToastNotificationManager, Windows.UI.Notifications, ContentType = WindowsRuntime] > $null; $template = [Windows.UI.Notifications.ToastNotificationManager]::GetTemplateContent([Windows.UI.Notifications.ToastTemplateType]::ToastText02); $text = $template.GetElementsByTagName('text'); $text.Item(0).AppendChild($template.CreateTextNode('{title}')) > $null; $text.Item(1).AppendChild($template.CreateTextNode('{body}')) > $null; [Windows.UI.Notifications.ToastNotificationManager]::CreateToastNotifier('Dial Control').Show([Windows.UI.Notifications.ToastNotification]::new($template));"#,
            title = title.replace('\'', "''"),
            body = body.replace('\'', "''")
        );
        let mut cmd = std::process::Command::new("powershell");
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
        let _ = cmd.args(["-NoProfile", "-NonInteractive", "-WindowStyle", "Hidden", "-Command", &script]).spawn();
    }

    #[cfg(target_os = "macos")]
    {
        let script = format!(
            r#"display notification "{body}" with title "{title}""#,
            title = title.replace('"', "\\\""),
            body = body.replace('"', "\\\"")
        );
        let _ = std::process::Command::new("osascript")
            .args(["-e", &script])
            .spawn();
    }
}

pub(crate) fn do_stop_scroll_mode(app: &AppHandle, state: &AppState) {
    let orig_vol = state.locked_vol.load(Ordering::SeqCst);
    state.is_scroll_mode.store(false, Ordering::SeqCst);
    state.locked_vol.store(-1, Ordering::SeqCst);
    state.is_hud_open.store(false, Ordering::SeqCst);
    if let Some(window) = app.get_webview_window("hud") {
        let _ = window.hide();
    }

    if orig_vol >= 0 {
        state.platform.audio().set_volume(orig_vol);
    }

    #[cfg(target_os = "windows")]
    {
        crate::platform::windows::dial::cleanup_hud_rotation();
    }

    let _ = app.emit("hud:visibility", false);
    let _ = app.emit("scroll_mode:exit", ());
    send_os_notification("Scroll Mode Exited", "Dial returned to standard mode");
    println!("[HUD] Stopped scroll mode");
}

#[tauri::command]
fn stop_scroll_mode(app: AppHandle, state: tauri::State<AppState>) {
    do_stop_scroll_mode(&app, &state);
}

#[tauri::command]
fn close_hud_for_scroll(app: AppHandle, state: tauri::State<AppState>) {
    state.is_hud_open.store(false, Ordering::SeqCst);
    state.is_scroll_mode.store(true, Ordering::SeqCst);

    return_focus_to_previous_app(&app);
    if let Some(window) = app.get_webview_window("hud") {
        let _ = window.hide();
    }
    let _ = app.emit("hud:visibility", false);

    send_os_notification(
        "Scroll Mode Active",
        "Rotate dial to scroll • Click dial or shortcut to exit",
    );

    println!("[HUD] Closed HUD window for scroll mode. Underlying application has full focus.");
}

pub(crate) fn do_show_hud(app: &AppHandle, state: &AppState) {
    // A second show (dial click racing the shortcut) must keep the original restore point.
    let already_locked = state.locked_vol.load(Ordering::SeqCst) >= 0;
    let curr_vol = capture_volume_lock(state);

    state.is_hud_open.store(true, Ordering::SeqCst);

    if !already_locked {
        #[cfg(target_os = "windows")]
        crate::platform::windows::dial::init_hud_rotation(curr_vol);
        #[cfg(target_os = "macos")]
        crate::platform::macos::dial::init_hud_rotation(curr_vol);
    }

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as usize;
    LAST_HUD_SHOWN_TIME.store(now, Ordering::SeqCst);

    if let Some(window) = app.get_webview_window("hud") {
        // Guarantee HUD window is centered on screen every time it is shown
        let _ = window.center();

        #[cfg(target_os = "linux")]
        {
            use gtk::prelude::*;
            if let Ok(gtk_w) = window.gtk_window() {
                gtk_w.set_position(gtk::WindowPosition::Center);
                gtk_w.set_accept_focus(true);
                gtk_w.set_focus_on_map(true);
            }
        }

        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
        let _ = window.set_always_on_top(true);

        #[cfg(target_os = "linux")]
        {
            use gtk::prelude::*;
            if let Ok(gtk_w) = window.gtk_window() {
                gtk_w.present();
            }
        }

        #[cfg(target_os = "windows")]
        {
            use crate::platform::windows::ffi::*;
            if let Ok(hwnd) = window.hwnd() {
                let hwnd = hwnd.0 as HWND;
                unsafe {
                    AllowSetForegroundWindow(GetCurrentProcessId());
                    ShowWindow(hwnd, SW_RESTORE);
                    SetForegroundWindow(hwnd);
                    SetWindowPos(
                        hwnd,
                        HWND_TOPMOST,
                        0,
                        0,
                        0,
                        0,
                        SWP_NOMOVE | SWP_NOSIZE | SWP_SHOWWINDOW,
                    );
                }
            }
        }
    }
    // Emitter::emit broadcasts to every webview regardless of the emitter it is called on,
    // so a second `window.emit` would deliver this event twice.
    let _ = app.emit("hud:visibility", true);
    let _ = app.emit(
        "sync:state",
        serde_json::json!({
            "volume": curr_vol,
        }),
    );
}

#[tauri::command]
fn show_hud(app: AppHandle, state: tauri::State<AppState>) {
    do_show_hud(&app, &state);
}

pub(crate) fn do_hide_hud(app: &AppHandle, state: &AppState) {
    let orig_vol = state.locked_vol.load(Ordering::SeqCst);
    state.is_hud_open.store(false, Ordering::SeqCst);
    state.is_scroll_mode.store(false, Ordering::SeqCst);
    state.locked_vol.store(-1, Ordering::SeqCst);
    return_focus_to_previous_app(app);
    if let Some(window) = app.get_webview_window("hud") {
        let _ = window.hide();
    }

    if orig_vol >= 0 {
        state.platform.audio().set_volume(orig_vol);
    }

    #[cfg(target_os = "windows")]
    {
        crate::platform::windows::dial::cleanup_hud_rotation();
    }

    let _ = app.emit("hud:visibility", false);
    let _ = app.emit("scroll_mode:exit", ());
}

#[tauri::command]
fn hide_hud(app: AppHandle, state: tauri::State<AppState>) {
    do_hide_hud(&app, &state);
}

#[tauri::command]
fn toggle_hud(app: AppHandle, state: tauri::State<AppState>) {
    if state.is_scroll_mode.load(Ordering::SeqCst) {
        do_stop_scroll_mode(&app, &state);
        return;
    }
    let currently_open = state.is_hud_open.load(Ordering::SeqCst);
    if currently_open {
        do_hide_hud(&app, &state);
    } else {
        do_show_hud(&app, &state);
    }
}

#[tauri::command]
fn show_settings(app: AppHandle) {
    if let Some(window) = app.get_webview_window("settings") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    } else {
        let builder = tauri::WebviewWindowBuilder::new(
            &app,
            "settings",
            tauri::WebviewUrl::App("index.html?view=settings".into()),
        )
        .title("Dial Control — Preferences")
        .inner_size(1180.0, 780.0)
        .min_inner_size(980.0, 620.0)
        .resizable(true)
        .decorations(true)
        .center();

        if let Ok(w) = builder.build() {
            if let Some(icon) = app.default_window_icon() {
                let _ = w.set_icon(icon.clone());
            }
        }
    }
}

#[tauri::command]
fn load_config(app: AppHandle) -> Option<serde_json::Value> {
    if let Ok(config_dir) = app.path().app_config_dir() {
        let config_path = config_dir.join("profiles.json");
        if config_path.exists() {
            if let Ok(content) = std::fs::read_to_string(&config_path) {
                if let Ok(val) = serde_json::from_str::<serde_json::Value>(&content) {
                    println!("[Config] Loaded profiles configuration from {:?}", config_path);
                    return Some(val);
                }
            }
        }
    }
    None
}

#[tauri::command]
fn save_config(app: AppHandle, config: serde_json::Value) -> Result<bool, String> {
    if let Ok(config_dir) = app.path().app_config_dir() {
        let _ = std::fs::create_dir_all(&config_dir);
        let config_path = config_dir.join("profiles.json");
        if let Ok(content) = serde_json::to_string_pretty(&config) {
            if let Err(e) = std::fs::write(&config_path, content) {
                eprintln!("[!] Failed to write config to {:?}: {:?}", config_path, e);
                return Err(e.to_string());
            }
            println!("[Config] Successfully saved profiles configuration to {:?}", config_path);
        }
    }

    // Broadcast config update to all webview windows (HUD, Preferences, etc.)
    let _ = app.emit("config:update", &config);
    Ok(true)
}

#[tauri::command]
fn execute_action(
    app: AppHandle,
    state: tauri::State<AppState>,
    action: serde_json::Value,
) {
    let action_id = action.get("id").and_then(|v| v.as_str()).unwrap_or("");
    let action_type = action.get("type").and_then(|v| v.as_str()).unwrap_or("");
    let action_key = action.get("action").and_then(|v| v.as_str()).unwrap_or("");

    println!("[Action] Executing action: {} (type: {}, action: {})", action_id, action_type, action_key);

    // Close / Dismiss HUD
    if action_id == "close_hud" || action_type == "close" {
        hide_hud(app, state);
        return;
    }

    if action_id == "mute_toggle" || action_type == "mute" || action_id == "video_mute" {
        state.platform.audio().toggle_mute();
        hide_hud(app, state);
        return;
    }

    if action_id == "media_playpause" || action_key == "playpause" {
        hide_hud(app, state.clone());
        let platform = state.platform.clone();
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(80));
            platform.media().play_pause();
        });
        return;
    }

    if action_id == "media_next" || action_key == "next" {
        hide_hud(app, state.clone());
        let platform = state.platform.clone();
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(80));
            platform.media().next_track();
        });
        return;
    }

    if action_id == "media_prev" || action_key == "prev" {
        hide_hud(app, state.clone());
        let platform = state.platform.clone();
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(80));
            platform.media().prev_track();
        });
        return;
    }

    match action_type {
        "hotkey" => {
            if let Some(keys_arr) = action.get("keys").and_then(|k| k.as_array()) {
                let keys: Vec<String> = keys_arr
                    .iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect();

                // 1. Hide HUD immediately so the compositor can return focus to the target window
                hide_hud(app, state.clone());

                // 2. Micro-sleep in background thread to guarantee target window focus settles before typing
                let platform = state.platform.clone();
                std::thread::spawn(move || {
                    std::thread::sleep(std::time::Duration::from_millis(120));
                    println!("[Action] Emitting hotkeys to foreground window: {:?}", keys);
                    platform.input().emit_hotkey(&keys);
                });
            }
        }
        "command" => {
            hide_hud(app, state);
            if action_id == "file_terminal" || action_id.contains("terminal") {
                println!("[Action] Spawning default terminal emulator...");
                #[cfg(target_os = "windows")]
                {
                    use std::os::windows::process::CommandExt;
                    let mut cmd = std::process::Command::new("cmd");
                    cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
                    let _ = cmd
                        .args(["/c", "where wt.exe >nul 2>&1 && start wt.exe || where powershell.exe >nul 2>&1 && start powershell.exe || start cmd.exe"])
                        .spawn();
                }
                #[cfg(target_os = "macos")]
                {
                    let _ = std::process::Command::new("sh")
                        .arg("-c")
                        .arg("open -a iTerm 2>/dev/null || open -a Terminal")
                        .spawn();
                }
                #[cfg(target_os = "linux")]
                {
                    let _ = std::process::Command::new("sh")
                        .arg("-c")
                        .arg("gtk-launch org.gnome.Ptyxis 2>/dev/null || gtk-launch org.gnome.Console 2>/dev/null || gtk-launch org.gnome.Terminal 2>/dev/null || x-terminal-emulator || ptyxis || gnome-terminal || kgx || konsole || alacritty || kitty || xfce4-terminal || foot || terminator || xterm")
                        .spawn();
                }
            } else if let Some(cmd) = action.get("cmd").and_then(|v| v.as_str()) {
                println!("[Action] Spawning command: {}", cmd);
                #[cfg(target_os = "windows")]
                {
                    use std::os::windows::process::CommandExt;
                    let mut proc = std::process::Command::new("cmd");
                    proc.creation_flags(0x08000000); // CREATE_NO_WINDOW
                    let _ = proc
                        .args(["/c", "start", cmd])
                        .spawn();
                }
                #[cfg(not(target_os = "windows"))]
                {
                    let _ = std::process::Command::new("sh")
                        .arg("-c")
                        .arg(cmd)
                        .spawn();
                }
            }
        }
        _ => {}
    }
}

#[tauri::command(async)]
fn adjust_gauge(
    app: AppHandle,
    state: tauri::State<AppState>,
    mode: String,
    delta: i32,
) {
    if mode == "brightness" {
        let dir = if delta > 0 { 1 } else { -1 };
        state.platform.brightness().step_brightness(dir);
    } else if mode == "volume" {
        let curr = state.platform.audio().get_volume();
        let next_val = (curr + delta).clamp(0, 100);
        state.platform.audio().set_volume(next_val);
    } else if mode == "scroll" {
        state.platform.input().emit_scroll(if delta > 0 { -1 } else { 1 });
        let _ = app.emit("scroll:tick", serde_json::json!({ "direction": delta }));
    }
}

#[cfg(target_os = "linux")]
fn convert_to_gnome_binding(shortcut: &str) -> String {
    let mut parts: Vec<String> = Vec::new();
    let tokens: Vec<&str> = shortcut.split('+').map(|s| s.trim()).collect();
    let mut key = String::new();
    for tok in tokens {
        let lower = tok.to_lowercase();
        match lower.as_str() {
            "super" | "meta" | "win" | "cmd" => parts.push("<Super>".to_string()),
            "ctrl" | "control" => parts.push("<Ctrl>".to_string()),
            "shift" => parts.push("<Shift>".to_string()),
            "alt" => parts.push("<Alt>".to_string()),
            other => key = other.to_string(),
        }
    }
    if key.is_empty() {
        return String::new();
    }
    let gnome_key = match key.to_lowercase().as_str() {
        "space" => "space".to_string(),
        "backslash" => "backslash".to_string(),
        "slash" => "slash".to_string(),
        "grave" | "`" => "grave".to_string(),
        "minus" | "-" => "minus".to_string(),
        "equal" | "=" => "equal".to_string(),
        "bracketleft" | "[" => "bracketleft".to_string(),
        "bracketright" | "]" => "bracketright".to_string(),
        "tab" => "Tab".to_string(),
        k => k.to_lowercase(),
    };
    format!("{}{}", parts.join(""), gnome_key)
}

#[cfg(target_os = "linux")]
fn sync_gnome_keybinding(shortcut: &str) {
    let path = "/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings/dial-control/";
    let schema = format!("org.gnome.settings-daemon.plugins.media-keys.custom-keybinding:{}", path);

    if shortcut.trim().is_empty() {
        let _ = std::process::Command::new("gsettings")
            .args(["set", &schema, "binding", ""])
            .status();
        return;
    }

    let gnome_binding = convert_to_gnome_binding(shortcut);
    if gnome_binding.is_empty() {
        return;
    }
    println!("[Shortcut] Synchronizing GNOME compositor keybinding: {}", gnome_binding);

    let _ = std::process::Command::new("gsettings")
        .args(["set", &schema, "name", "Dial Control Radial HUD"])
        .status();

    let exe_path = std::env::current_exe()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| "dial-control".to_string());
    let exec_cmd = format!("{} --toggle", exe_path);
    let _ = std::process::Command::new("gsettings")
        .args(["set", &schema, "command", &exec_cmd])
        .status();

    let _ = std::process::Command::new("gsettings")
        .args(["set", &schema, "binding", &gnome_binding])
        .status();

    if let Ok(out) = std::process::Command::new("gsettings")
        .args(["get", "org.gnome.settings-daemon.plugins.media-keys", "custom-keybindings"])
        .output()
    {
        let current_list = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if !current_list.contains(path) {
            let list_val = if current_list.is_empty() || current_list == "@as []" || current_list == "[]" {
                format!("['{}']", path)
            } else if current_list.ends_with(']') {
                let trimmed = current_list.trim_end_matches(']').trim();
                if trimmed.ends_with('[') {
                    format!("['{}']", path)
                } else {
                    format!("{}, '{}']", trimmed, path)
                }
            } else {
                format!("['{}']", path)
            };
            let _ = std::process::Command::new("gsettings")
                .args(["set", "org.gnome.settings-daemon.plugins.media-keys", "custom-keybindings", &list_val])
                .status();
        }
    }
}

#[tauri::command]
fn register_activation_shortcut(app: AppHandle, shortcut_str: String) -> Result<String, String> {
    let clean = shortcut_str.trim().to_string();

    #[cfg(target_os = "linux")]
    {
        sync_gnome_keybinding(&clean);
    }

    use std::str::FromStr;
    use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut};

    let global_shortcut = app.global_shortcut();
    let _ = global_shortcut.unregister_all();

    if clean.is_empty() {
        return Ok("Disabled".to_string());
    }

    match Shortcut::from_str(&clean) {
        Ok(s) => match global_shortcut.register(s) {
            Ok(_) => {
                println!("[Shortcut] Successfully registered global activation shortcut: {}", clean);
                Ok(clean)
            }
            Err(e) => {
                println!("[Shortcut] Global shortcut plugin fallback: {:?}", e);
                Ok(clean)
            }
        },
        Err(e) => {
            println!("[Shortcut] Global shortcut parse fallback: {:?}", e);
            Ok(clean)
        }
    }
}

#[tauri::command]
fn set_recording_shortcut(state: tauri::State<AppState>, recording: bool) {
    state.is_recording_shortcut.store(recording, Ordering::SeqCst);
    println!("[Shortcut] Recording shortcut active state: {}", recording);
}

pub fn run() {
    let is_hud_open = Arc::new(AtomicBool::new(false));
    let locked_vol = Arc::new(AtomicI32::new(-1));
    let is_recording_shortcut = Arc::new(AtomicBool::new(false));
    let is_scroll_mode = Arc::new(AtomicBool::new(false));
    let platform = Arc::new(Platform::new());

    let app_state = AppState {
        platform: platform.clone(),
        is_hud_open: is_hud_open.clone(),
        locked_vol: locked_vol.clone(),
        is_recording_shortcut: is_recording_shortcut.clone(),
        is_scroll_mode: is_scroll_mode.clone(),
    };

    tauri::Builder::default()
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _shortcut, event| {
                    if event.state() == tauri_plugin_global_shortcut::ShortcutState::Pressed {
                        let state: tauri::State<AppState> = app.state();
                        toggle_hud(app.clone(), state);
                    }
                })
                .build(),
        )
        .manage(app_state)
        .on_window_event(|window, event| {
            match event {
                tauri::WindowEvent::CloseRequested { api, .. } => {
                    let state: tauri::State<AppState> = window.state();
                    if state.is_recording_shortcut.load(Ordering::SeqCst) {
                        println!("[Window] CloseRequested suppressed because shortcut recording is in progress");
                        api.prevent_close();
                        let _ = window.emit("recording:system_close_pressed", ());
                        return;
                    }
                    if window.label() == "settings" || window.label() == "hud" {
                        let _ = window.hide();
                        api.prevent_close();
                    }
                }
                tauri::WindowEvent::Focused(false) => {
                    if window.label() == "hud" {
                        let now = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_millis() as usize;
                        let shown_time = LAST_HUD_SHOWN_TIME.load(Ordering::SeqCst);
                        // 400ms grace period so initial presentation focus transition does not auto-dismiss
                        if now.saturating_sub(shown_time) > 400 {
                            let app = window.app_handle().clone();
                            let state: tauri::State<AppState> = app.state();
                            if !state.is_scroll_mode.load(Ordering::SeqCst) && state.is_hud_open.load(Ordering::SeqCst) {
                                println!("[Window] HUD lost focus -> auto-dismissing");
                                hide_hud(app.clone(), state);
                            }
                        }
                    }
                }
                _ => {}
            }
        })
        .setup(move |app| {
            // Build Tray Menu (Explicit MenuItems with proper labels and separator)
            let toggle_i = MenuItem::with_id(app, "toggle_hud", "Toggle Radial HUD", true, None::<&str>)?;
            let settings_i = MenuItem::with_id(app, "open_settings", "Settings and Preferences", true, None::<&str>)?;
            let sep = PredefinedMenuItem::separator(app)?;
            let quit_i = MenuItem::with_id(app, "quit", "Quit Dial Control", true, None::<&str>)?;

            let menu = Menu::with_items(app, &[&toggle_i, &settings_i, &sep, &quit_i])?;

            // Register default global activation shortcut (Super+Shift+D) for users without dial hardware
            use std::str::FromStr;
            use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut};
            if let Ok(default_shortcut) = Shortcut::from_str("Super+Shift+D") {
                let _ = app.global_shortcut().register(default_shortcut);
            }

            // Sync with GNOME on Linux Wayland
            #[cfg(target_os = "linux")]
            {
                sync_gnome_keybinding("Super+Shift+D");
                crate::platform::linux::context::ensure_bundled_extension_installed();
            }

            // UNIX socket listener for instantaneous CLI --toggle on Wayland
            #[cfg(unix)]
            {
                let app_handle = app.handle().clone();
                let socket_path = "/tmp/dial-control.sock";
                let _ = std::fs::remove_file(socket_path);
                if let Ok(listener) = std::os::unix::net::UnixListener::bind(socket_path) {
                    use std::io::{BufRead, BufReader};
                    std::thread::spawn(move || {
                        for stream in listener.incoming() {
                            if let Ok(stream) = stream {
                                let mut reader = BufReader::new(stream);
                                let mut line = String::new();
                                if reader.read_line(&mut line).is_ok() {
                                    let cmd = line.trim();
                                    if cmd == "toggle" {
                                        let state: tauri::State<AppState> = app_handle.state();
                                        toggle_hud(app_handle.clone(), state);
                                    }
                                }
                            }
                        }
                    });
                    println!("[IPC] UNIX socket listener active on {}", socket_path);
                }
            }

            // TCP loopback listener for instantaneous CLI --toggle on Windows
            #[cfg(windows)]
            {
                let app_handle = app.handle().clone();
                if let Ok(listener) = std::net::TcpListener::bind("127.0.0.1:41829") {
                    use std::io::{BufRead, BufReader};
                    std::thread::spawn(move || {
                        for stream in listener.incoming() {
                            if let Ok(stream) = stream {
                                let mut reader = BufReader::new(stream);
                                let mut line = String::new();
                                if reader.read_line(&mut line).is_ok() {
                                    let cmd = line.trim();
                                    if cmd == "toggle" {
                                        let state: tauri::State<AppState> = app_handle.state();
                                        toggle_hud(app_handle.clone(), state);
                                    }
                                }
                            }
                        }
                    });
                    println!("[IPC] TCP loopback listener active on 127.0.0.1:41829");
                }
            }

            // Set app window icons on taskbar/dock
            if let Some(icon) = app.default_window_icon() {
                if let Some(settings_w) = app.get_webview_window("settings") {
                    let _ = settings_w.set_icon(icon.clone());
                }
                if let Some(hud_w) = app.get_webview_window("hud") {
                    let _ = hud_w.set_icon(icon.clone());
                    let _ = hud_w.set_shadow(false);
                }
            }

            // Build Tray Icon
            let _tray = TrayIconBuilder::with_id("main_tray")
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .tooltip("Dial Control — Active")
                .show_menu_on_left_click(true)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "toggle_hud" => {
                        let state: tauri::State<AppState> = app.state();
                        toggle_hud(app.clone(), state);
                    }
                    "open_settings" => {
                        show_settings(app.clone());
                    }
                    "quit" => {
                        app.exit(0);
                    }
                    _ => {}
                })
                .build(app)?;

            // Start hardware background threads via unified platform abstractions
            platform.dial().start_listeners(
                app.handle().clone(),
                is_hud_open.clone(),
                locked_vol.clone(),
                is_scroll_mode.clone(),
            );
            platform.context().start(app.handle().clone());

            println!("[+] Dial Control Daemon initialized with modular platform provider.");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            toggle_hud,
            show_hud,
            hide_hud,
            show_settings,
            execute_action,
            adjust_gauge,
            get_system_brightness,
            set_system_brightness,
            step_brightness,
            get_system_volume,
            set_system_volume,
            set_mode,
            stop_scroll_mode,
            close_hud_for_scroll,
            register_activation_shortcut,
            set_recording_shortcut,
            load_config,
            save_config
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

