pub mod virtual_input;
pub mod dial_backend;
pub mod context_detector;

use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use std::sync::Arc;
use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    AppHandle, Emitter, Manager,
};
use virtual_input::VirtualKeyboard;

pub struct AppState {
    pub vk: VirtualKeyboard,
    pub is_hud_open: Arc<AtomicBool>,
    pub locked_vol: Arc<AtomicI32>,
}

struct BacklightInfo {
    serial: u32,
    connector: String,
    min: i32,
    max: i32,
    value: i32,
}

fn get_backlight_info() -> Option<BacklightInfo> {
    let out = std::process::Command::new("busctl")
        .args([
            "--user",
            "get-property",
            "org.gnome.Mutter.DisplayConfig",
            "/org/gnome/Mutter/DisplayConfig",
            "org.gnome.Mutter.DisplayConfig",
            "Backlight",
        ])
        .output()
        .ok()?;

    if !out.status.success() {
        return None;
    }

    let text = String::from_utf8_lossy(&out.stdout);
    let parts: Vec<&str> = text.split_whitespace().collect();
    let mut serial: u32 = 4;
    let mut connector = "eDP-1".to_string();
    let mut min: Option<i32> = None;
    let mut max: Option<i32> = None;
    let mut value: Option<i32> = None;

    if let Some(s_idx) = parts.iter().position(|&p| p == "(uaa{sv})") {
        if let Some(s) = parts.get(s_idx + 1).and_then(|p| p.parse::<u32>().ok()) {
            serial = s;
        }
    }
    for (i, &p) in parts.iter().enumerate() {
        if p == "\"connector\"" && i + 2 < parts.len() {
            connector = parts[i + 2].trim_matches('"').to_string();
        } else if p == "\"min\"" && i + 2 < parts.len() {
            if let Ok(v) = parts[i + 2].parse::<i32>() {
                min = Some(v);
            }
        } else if p == "\"max\"" && i + 2 < parts.len() {
            if let Ok(v) = parts[i + 2].parse::<i32>() {
                max = Some(v);
            }
        } else if p == "\"value\"" && i + 2 < parts.len() {
            if let Ok(v) = parts[i + 2].parse::<i32>() {
                value = Some(v);
            }
        }
    }

    if let (Some(min), Some(max), Some(value)) = (min, max, value) {
        Some(BacklightInfo {
            serial,
            connector,
            min,
            max,
            value,
        })
    } else {
        None
    }
}

fn get_sysfs_backlight() -> Option<i32> {
    let actual_str = std::fs::read_to_string("/sys/class/backlight/intel_backlight/actual_brightness")
        .or_else(|_| std::fs::read_to_string("/sys/class/backlight/amdgpu_bl0/actual_brightness"))
        .ok()?;
    let max_str = std::fs::read_to_string("/sys/class/backlight/intel_backlight/max_brightness")
        .or_else(|_| std::fs::read_to_string("/sys/class/backlight/amdgpu_bl0/max_brightness"))
        .ok()?;
    let actual: f64 = actual_str.trim().parse().ok()?;
    let max: f64 = max_str.trim().parse().ok()?;
    if max > 0.0 {
        Some(((actual / max) * 100.0).round().clamp(0.0, 100.0) as i32)
    } else {
        None
    }
}

fn get_mutter_resources_brightness() -> Option<i32> {
    let out = std::process::Command::new("busctl")
        .args([
            "--user",
            "call",
            "org.gnome.Mutter.DisplayConfig",
            "/org/gnome/Mutter/DisplayConfig",
            "org.gnome.Mutter.DisplayConfig",
            "GetResources",
        ])
        .output()
        .ok()?;

    if !out.status.success() {
        return None;
    }

    let text = String::from_utf8_lossy(&out.stdout);
    let parts: Vec<&str> = text.split_whitespace().collect();
    for (i, &p) in parts.iter().enumerate() {
        if p == "\"backlight\"" && i + 2 < parts.len() {
            if let Ok(val) = parts[i + 2].parse::<i32>() {
                if val >= 0 {
                    return Some(val.clamp(0, 100));
                }
            }
        }
    }
    None
}

#[tauri::command]
fn get_system_brightness() -> i32 {
    // 1. Try GNOME Mutter GetResources (direct native brightness percentage 0-100%)
    if let Some(pct) = get_mutter_resources_brightness() {
        println!("[Brightness] Read from Mutter GetResources: {}%", pct);
        return pct;
    }
    // 2. Try Mutter Backlight property
    if let Some(b) = get_backlight_info() {
        let range = (b.max - b.min).max(1);
        let pct = (((b.value - b.min) as f64 / range as f64) * 100.0).round() as i32;
        return pct.clamp(0, 100);
    }
    // 3. Fallback to sysfs
    if let Some(pct) = get_sysfs_backlight() {
        return pct;
    }
    50
}

#[tauri::command]
fn step_brightness(state: tauri::State<AppState>, direction: i32) {
    if direction > 0 {
        state.vk.emit_brightness_up();
        println!("[Brightness] Emitted KEY_BRIGHTNESSUP (Fn+F6)");
    } else if direction < 0 {
        state.vk.emit_brightness_down();
        println!("[Brightness] Emitted KEY_BRIGHTNESSDOWN (Fn+F5)");
    }
}

#[tauri::command]
fn set_system_brightness(_state: tauri::State<AppState>, pct: i32) -> i32 {
    let pct = pct.clamp(0, 100);
    if let Some(b) = get_backlight_info() {
        let range = b.max - b.min;
        let raw = b.min + (pct * range) / 100;
        let _ = std::process::Command::new("busctl")
            .args([
                "--user",
                "call",
                "org.gnome.Mutter.DisplayConfig",
                "/org/gnome/Mutter/DisplayConfig",
                "org.gnome.Mutter.DisplayConfig",
                "SetBacklight",
                "usi",
                &b.serial.to_string(),
                &b.connector,
                &raw.to_string(),
            ])
            .status();
        println!("[Brightness] Set to {}% (raw: {}/{}) on {}", pct, raw, b.max, b.connector);
    }
    pct
}

#[tauri::command]
fn get_system_volume() -> i32 {
    // 1. Try wpctl (native WirePlumber)
    if let Ok(output) = std::process::Command::new("wpctl")
        .args(["get-volume", "@DEFAULT_AUDIO_SINK@"])
        .output()
    {
        let text = String::from_utf8_lossy(&output.stdout);
        for part in text.split_whitespace() {
            if let Ok(val) = part.parse::<f64>() {
                return ((val * 100.0).round() as i32).clamp(0, 100);
            }
        }
    }
    // 2. Fallback to pactl
    if let Ok(output) = std::process::Command::new("pactl")
        .args(["get-sink-volume", "@DEFAULT_SINK@"])
        .output()
    {
        let text = String::from_utf8_lossy(&output.stdout);
        for part in text.split_whitespace() {
            if part.ends_with('%') {
                if let Ok(v) = part.trim_end_matches('%').parse::<i32>() {
                    return v.clamp(0, 100);
                }
            }
        }
    }
    50
}

#[tauri::command]
fn set_system_volume(pct: i32) -> i32 {
    let pct = pct.clamp(0, 100);
    let frac = format!("{:.2}", (pct as f64) / 100.0);
    let _ = std::process::Command::new("wpctl")
        .args(["set-volume", "-l", "1.0", "@DEFAULT_AUDIO_SINK@", &frac])
        .status();
    let _ = std::process::Command::new("pactl")
        .args(["set-sink-volume", "@DEFAULT_SINK@", &format!("{}%", pct)])
        .status();
    println!("[Volume] Set to {}% (wpctl: {})", pct, frac);
    pct
}

#[tauri::command]
fn set_mode(state: tauri::State<AppState>, mode: String) {
    if mode == "volume" {
        state.locked_vol.store(0, Ordering::SeqCst);
        println!("[HUD] Switched to volume mode (lock released for natural hardware volume control)");
    } else {
        state.locked_vol.store(50, Ordering::SeqCst);
        println!("[HUD] Switched to mode: {} (volume locked at 50 for navigation)", mode);
    }
}

fn control_mpris_or_fallback(method: &str, fallback_vk: impl FnOnce()) {
    let script = format!(
        r#"
for p in $(busctl --user list 2>/dev/null | awk '/org\.mpris\.MediaPlayer2/ {{print $1}}'); do
    if busctl --user call "$p" /org/mpris/MediaPlayer2 org.mpris.MediaPlayer2.Player "{}" 2>/dev/null; then
        exit 0
    fi
done
exit 1
"#,
        method
    );
    let success = std::process::Command::new("sh")
        .arg("-c")
        .arg(script)
        .status()
        .map(|s| s.success())
        .unwrap_or(false);

    if !success {
        println!("[Media] MPRIS player not reachable, invoking virtual keyboard fallback");
        fallback_vk();
    }
}

#[tauri::command]
fn show_hud(app: AppHandle, state: tauri::State<AppState>) {
    state.is_hud_open.store(true, Ordering::SeqCst);
    state.locked_vol.store(50, Ordering::SeqCst);
    if let Some(window) = app.get_webview_window("hud") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
        let _ = window.set_always_on_top(true);
    }
    let _ = app.emit("hud:visibility", true);
}

#[tauri::command]
fn hide_hud(app: AppHandle, state: tauri::State<AppState>) {
    state.is_hud_open.store(false, Ordering::SeqCst);
    state.locked_vol.store(0, Ordering::SeqCst);
    if let Some(window) = app.get_webview_window("hud") {
        let _ = window.hide();
    }
    let _ = app.emit("hud:visibility", false);
}

#[tauri::command]
fn toggle_hud(app: AppHandle, state: tauri::State<AppState>) {
    let currently_open = state.is_hud_open.load(Ordering::SeqCst);
    if currently_open {
        hide_hud(app, state);
    } else {
        show_hud(app, state);
    }
}

#[tauri::command]
fn show_settings(app: AppHandle) {
    if let Some(window) = app.get_webview_window("settings") {
        let _ = window.show();
        let _ = window.set_focus();
    }
}

#[tauri::command]
fn execute_action(
    state: tauri::State<AppState>,
    action: serde_json::Value,
) {
    let action_id = action.get("id").and_then(|v| v.as_str()).unwrap_or("");
    let action_type = action.get("type").and_then(|v| v.as_str()).unwrap_or("");
    let action_key = action.get("action").and_then(|v| v.as_str()).unwrap_or("");

    println!("[Action] Executing action: {} (type: {}, action: {})", action_id, action_type, action_key);

    if action_id == "mute_toggle" || action_type == "mute" || action_id == "video_mute" {
        let _ = std::process::Command::new("wpctl")
            .args(["set-mute", "@DEFAULT_AUDIO_SINK@", "toggle"])
            .status();
        println!("[Audio] Toggled sink mute via wpctl.");
        return;
    }

    if action_id == "media_playpause" || action_key == "playpause" {
        println!("[Action] Play/Pause: sending MPRIS or fallback");
        control_mpris_or_fallback("PlayPause", || state.vk.emit_play_pause());
        return;
    }

    if action_id == "media_next" || action_key == "next" {
        println!("[Action] Next track: sending MPRIS or fallback");
        control_mpris_or_fallback("Next", || state.vk.emit_next_track());
        return;
    }

    if action_id == "media_prev" || action_key == "prev" {
        println!("[Action] Previous track: sending MPRIS or fallback");
        control_mpris_or_fallback("Previous", || state.vk.emit_prev_track());
        return;
    }

    match action_type {
        "hotkey" => {
            if let Some(keys_arr) = action.get("keys").and_then(|k| k.as_array()) {
                let keys: Vec<String> = keys_arr
                    .iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect();
                println!("[Action] Emitting hotkeys: {:?}", keys);
                state.vk.emit_key_combination(&keys);
            }
        }
        "command" => {
            if let Some(cmd) = action.get("cmd").and_then(|v| v.as_str()) {
                println!("[Action] Spawning command: {}", cmd);
                let _ = std::process::Command::new("sh")
                    .arg("-c")
                    .arg(cmd)
                    .spawn();
            }
        }
        _ => {}
    }
}

#[tauri::command]
fn adjust_gauge(
    state: tauri::State<AppState>,
    mode: String,
    delta: i32,
) {
    if mode == "brightness" {
        let dir = if delta > 0 { 1 } else { -1 };
        step_brightness(state, dir);
    } else if mode == "volume" {
        let curr = get_system_volume();
        let next_val = (curr + delta).clamp(0, 100);
        set_system_volume(next_val);
    }
}

pub fn run() {
    let is_hud_open = Arc::new(AtomicBool::new(false));
    let locked_vol = Arc::new(AtomicI32::new(0));
    let vk = VirtualKeyboard::new();

    let app_state = AppState {
        vk,
        is_hud_open: is_hud_open.clone(),
        locked_vol: locked_vol.clone(),
    };

    tauri::Builder::default()
        .manage(app_state)
        .setup(move |app| {
            // Build Tray Menu
            let toggle_i = MenuItem::with_id(app, "toggle_hud", "🔘 Show / Hide Radial HUD", true, None::<&str>)?;
            let settings_i = MenuItem::with_id(app, "open_settings", "⚙️ Settings & Customizer...", true, None::<&str>)?;
            let quit_i = MenuItem::with_id(app, "quit", "❌ Quit Dial Control", true, None::<&str>)?;

            let menu = Menu::with_items(app, &[&toggle_i, &settings_i, &quit_i])?;

            // Build Tray Icon
            let _tray = TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
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

            // Start hardware background threads
            let handle_grabber = app.handle().clone();
            let handle_bluez = app.handle().clone();
            let handle_context = app.handle().clone();

            dial_backend::start_input_grabber(handle_grabber, is_hud_open.clone(), locked_vol.clone());
            dial_backend::start_bluez_listener(handle_bluez, is_hud_open.clone(), locked_vol.clone());
            context_detector::start_context_detector(handle_context);

            println!("[+] Dial Control Tauri v2 Daemon initialized with BlueZ & evdev bridge.");
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
            set_mode
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
