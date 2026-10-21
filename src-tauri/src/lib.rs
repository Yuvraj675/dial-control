pub mod platform;

use platform::{Platform, PlatformProvider};
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use std::sync::Arc;
use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    AppHandle, Emitter, Manager,
};

pub struct AppState {
    pub platform: Arc<Platform>,
    pub is_hud_open: Arc<AtomicBool>,
    pub locked_vol: Arc<AtomicI32>,
}

#[tauri::command]
fn get_system_brightness(state: tauri::State<AppState>) -> i32 {
    state.platform.brightness().get_brightness()
}

#[tauri::command]
fn set_system_brightness(state: tauri::State<AppState>, pct: i32) -> i32 {
    state.platform.brightness().set_brightness(pct)
}

#[tauri::command]
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
        let _ = window.unminimize();
        let _ = window.set_focus();
    } else {
        let _ = tauri::WebviewWindowBuilder::new(
            &app,
            "settings",
            tauri::WebviewUrl::App("index.html?view=settings".into()),
        )
        .title("Dial Control — Preferences")
        .inner_size(1080.0, 680.0)
        .min_inner_size(900.0, 550.0)
        .resizable(true)
        .decorations(true)
        .center()
        .build();
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
        state.platform.audio().toggle_mute();
        return;
    }

    if action_id == "media_playpause" || action_key == "playpause" {
        state.platform.media().play_pause();
        return;
    }

    if action_id == "media_next" || action_key == "next" {
        state.platform.media().next_track();
        return;
    }

    if action_id == "media_prev" || action_key == "prev" {
        state.platform.media().prev_track();
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
                state.platform.input().emit_hotkey(&keys);
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
        state.platform.brightness().step_brightness(dir);
    } else if mode == "volume" {
        let curr = state.platform.audio().get_volume();
        let next_val = (curr + delta).clamp(0, 100);
        state.platform.audio().set_volume(next_val);
    }
}

pub fn run() {
    let is_hud_open = Arc::new(AtomicBool::new(false));
    let locked_vol = Arc::new(AtomicI32::new(0));
    let platform = Arc::new(Platform::new());

    let app_state = AppState {
        platform: platform.clone(),
        is_hud_open: is_hud_open.clone(),
        locked_vol: locked_vol.clone(),
    };

    tauri::Builder::default()
        .manage(app_state)
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                if window.label() == "settings" || window.label() == "hud" {
                    let _ = window.hide();
                    api.prevent_close();
                }
            }
        })
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

            // Start hardware background threads via unified platform abstractions
            platform.dial().start_listeners(app.handle().clone(), is_hud_open.clone(), locked_vol.clone());
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
            set_mode
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
