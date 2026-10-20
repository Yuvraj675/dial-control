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

#[tauri::command]
fn show_hud(app: AppHandle, state: tauri::State<AppState>) {
    state.is_hud_open.store(true, Ordering::SeqCst);
    state.locked_vol.store(50, Ordering::SeqCst);
    if let Some(window) = app.get_webview_window("hud") {
        let _ = window.show();
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

    println!("[Action] Executing action: {} (type: {})", action_id, action_type);

    match action_type {
        "hotkey" => {
            if let Some(keys_arr) = action.get("keys").and_then(|k| k.as_array()) {
                let keys: Vec<String> = keys_arr
                    .iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect();
                state.vk.emit_key_combination(&keys);
            }
        }
        "media" => {
            match action_id {
                "media_playpause" => state.vk.emit_play_pause(),
                "media_next" => state.vk.emit_next_track(),
                "media_prev" => state.vk.emit_prev_track(),
                _ => {}
            }
        }
        "command" => {
            if let Some(cmd) = action.get("cmd").and_then(|v| v.as_str()) {
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
        if delta > 0 {
            state.vk.emit_brightness_up();
        } else {
            state.vk.emit_brightness_down();
        }
    } else if mode == "volume" {
        let sign = if delta > 0 { "+" } else { "-" };
        let abs_delta = delta.abs();
        let _ = std::process::Command::new("pactl")
            .args(["set-sink-volume", "@DEFAULT_SINK@", &format!("{}{}%", sign, abs_delta)])
            .status();
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

            dial_backend::start_input_grabber(handle_grabber, is_hud_open.clone());
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
            adjust_gauge
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
