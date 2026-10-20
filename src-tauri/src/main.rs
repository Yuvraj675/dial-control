// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    AppHandle, Manager,
};

#[tauri::command]
fn toggle_hud(app: AppHandle) {
    if let Some(window) = app.get_webview_window("hud") {
        if window.is_visible().unwrap_or(false) {
            let _ = window.hide();
        } else {
            let _ = window.show();
        }
    }
}

#[tauri::command]
fn show_settings(app: AppHandle) {
    if let Some(window) = app.get_webview_window("settings") {
        let _ = window.show();
        let _ = window.set_focus();
    }
}

fn main() {
    tauri::Builder::default()
        .setup(|app| {
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
                        toggle_hud(app.clone());
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

            println!("[+] Dial Control Tauri v2 Daemon initialized.");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![toggle_hud, show_settings])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
