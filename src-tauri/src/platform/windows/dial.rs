use crate::platform::traits::{DialBackend, PlatformProvider};
use crate::platform::windows::ffi::*;
use crate::AppState;
use std::ptr::null_mut;
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicUsize, Ordering};
use std::sync::{Arc, OnceLock};
use std::thread;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};

static IS_HUD_OPEN_REF: OnceLock<Arc<AtomicBool>> = OnceLock::new();
static LOCKED_VOL_REF: OnceLock<Arc<AtomicI32>> = OnceLock::new();
static IS_SCROLL_MODE_REF: OnceLock<Arc<AtomicBool>> = OnceLock::new();
static APP_HANDLE_REF: OnceLock<AppHandle> = OnceLock::new();
static HOOK_HANDLE: AtomicUsize = AtomicUsize::new(0);
static LAST_DIAL_CLICK_TIME: AtomicUsize = AtomicUsize::new(0);

#[derive(Clone, serde::Serialize)]
struct RotatePayload {
    direction: i32,
    delta: i32,
    volume: i32,
    volume_pct: i32,
}

unsafe extern "system" fn low_level_keyboard_proc(
    n_code: i32,
    w_param: WPARAM,
    l_param: LPARAM,
) -> LRESULT {
    let hhook = HOOK_HANDLE.load(Ordering::SeqCst) as HHOOK;

    if n_code >= 0 && l_param != 0 {
        let kbd = *(l_param as *const KBDLLHOOKSTRUCT);

        // Passthrough if the input was synthesized by Dial Control itself
        if kbd.dwExtraInfo == SYNTHETIC_INPUT_TAG {
            return CallNextHookEx(hhook, n_code, w_param, l_param);
        }

        let vk = kbd.vkCode as u16;
        let is_key_down = w_param == WM_KEYDOWN as usize || w_param == WM_SYSKEYDOWN as usize;
        let is_key_up = w_param == WM_KEYUP as usize || w_param == WM_SYSKEYUP as usize;

        let is_hud_open = IS_HUD_OPEN_REF
            .get()
            .map(|arc| arc.load(Ordering::SeqCst))
            .unwrap_or(false);

        let is_scroll_mode = IS_SCROLL_MODE_REF
            .get()
            .map(|arc| arc.load(Ordering::SeqCst))
            .unwrap_or(false);

        match vk {
            VK_MEDIA_PLAY_PAUSE | VK_PAUSE | VK_MEDIA_STOP => {
                // IMPORTANT: Always consume keyup for intercepted media keys so underlying
                // media players (Spotify, YouTube, Chrome) never receive a play/pause toggle on key release!
                if is_key_up {
                    return 1;
                }

                if is_key_down {
                    let now = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_millis() as usize;

                    let last = LAST_DIAL_CLICK_TIME.load(Ordering::SeqCst);
                    if now.saturating_sub(last) < 250 {
                        // Hardware contact debounce: block repeat event
                        return 1;
                    }
                    LAST_DIAL_CLICK_TIME.store(now, Ordering::SeqCst);

                    if is_scroll_mode {
                        println!("[Windows Dial] Intercepted Dial Click -> Exiting Scroll Mode");
                        if let Some(arc) = IS_SCROLL_MODE_REF.get() {
                            arc.store(false, Ordering::SeqCst);
                        }
                        if let Some(arc) = LOCKED_VOL_REF.get() {
                            arc.store(-1, Ordering::SeqCst);
                        }
                        if let Some(arc) = IS_HUD_OPEN_REF.get() {
                            arc.store(false, Ordering::SeqCst);
                        }
                        if let Some(app) = APP_HANDLE_REF.get() {
                            let app = app.clone();
                            thread::spawn(move || {
                                let state: tauri::State<AppState> = app.state();
                                crate::do_stop_scroll_mode(&app, &state);
                            });
                        }
                        return 1;
                    }

                    if !is_hud_open {
                        // 1. Immediately flip atomic flag so consecutive inputs know HUD is active
                        if let Some(arc) = IS_HUD_OPEN_REF.get() {
                            arc.store(true, Ordering::SeqCst);
                        }

                        // 2. Offload GUI display to worker thread so the low-level hook returns in <0.01ms,
                        // preventing Windows from timing out and silently removing the hook!
                        if let Some(app) = APP_HANDLE_REF.get() {
                            let app = app.clone();
                            thread::spawn(move || {
                                println!("[Windows Dial] Intercepted Earbuds Dial Click -> Opening HUD");
                                let state: tauri::State<AppState> = app.state();
                                crate::do_show_hud(&app, &state);
                            });
                        }
                        // Block OS and underlying media apps from toggling audio playback
                        return 1;
                    } else {
                        println!("[Windows Dial] Intercepted Earbuds Dial Click -> Confirm selection");
                        if let Some(app) = APP_HANDLE_REF.get() {
                            let app = app.clone();
                            thread::spawn(move || {
                                let _ = app.emit("dial:click", ());
                                if let Some(window) = app.get_webview_window("hud") {
                                    let _ = window.emit("dial:click", ());
                                }
                            });
                        }
                        // Block OS and underlying media apps from toggling audio playback
                        return 1;
                    }
                }
            }
            VK_VOLUME_UP | VK_VOLUME_DOWN if (is_hud_open || is_scroll_mode) => {
                // Swallowing keyup prevents Windows from popping up the OS volume slider on release
                if is_key_up {
                    return 1;
                }

                if is_key_down {
                    let dir = if vk == VK_VOLUME_UP { 1 } else { -1 };

                    if is_scroll_mode {
                        // Clockwise rotation (dir = 1) -> Scroll DOWN (-1)
                        // Counterclockwise rotation (dir = -1) -> Scroll UP (1)
                        let scroll_delta = if dir > 0 { -1 } else { 1 };
                        println!("[Windows Dial] Scroll Mode Emulation -> dir: {}, delta: {}", dir, scroll_delta);
                        if let Some(app) = APP_HANDLE_REF.get() {
                            let app = app.clone();
                            thread::spawn(move || {
                                let state: tauri::State<AppState> = app.state();
                                state.platform.input().emit_scroll(scroll_delta);
                                let _ = app.emit("scroll:tick", serde_json::json!({ "direction": dir }));
                            });
                        }
                        return 1;
                    }

                    println!("[Windows Dial] Intercepted Dial Rotate -> ({})", dir);

                    let locked_val = LOCKED_VOL_REF
                        .get()
                        .map(|arc| arc.load(Ordering::SeqCst))
                        .unwrap_or(-1);

                    if let Some(app) = APP_HANDLE_REF.get() {
                        let app = app.clone();
                        thread::spawn(move || {
                            let state: tauri::State<AppState> = app.state();
                            let current_vol = state.platform.audio().get_volume();

                            let new_vol = if locked_val < 0 {
                                state.platform.audio().set_volume((current_vol + dir * 2).clamp(0, 100))
                            } else {
                                current_vol
                            };

                            let payload = RotatePayload {
                                direction: dir,
                                delta: dir,
                                volume: new_vol,
                                volume_pct: new_vol,
                            };
                            let _ = app.emit("dial:rotate", payload.clone());
                            if let Some(window) = app.get_webview_window("hud") {
                                let _ = window.emit("dial:rotate", payload);
                            }
                        });
                    }
                    // Block volume key from default OS beep/volume popup while navigating
                    return 1;
                }
            }
            _ => {}
        }
    }

    CallNextHookEx(hhook, n_code, w_param, l_param)
}

pub struct WindowsDial;

impl WindowsDial {
    pub fn new() -> Self {
        Self
    }
}

impl DialBackend for WindowsDial {
    fn start_listeners(
        &self,
        app: AppHandle,
        is_hud_open: Arc<AtomicBool>,
        locked_vol: Arc<AtomicI32>,
        is_scroll_mode: Arc<AtomicBool>,
    ) {
        let _ = IS_HUD_OPEN_REF.set(is_hud_open);
        let _ = LOCKED_VOL_REF.set(locked_vol);
        let _ = IS_SCROLL_MODE_REF.set(is_scroll_mode);
        let _ = APP_HANDLE_REF.set(app);

        // Spawn a dedicated thread with a Win32 message pump for the low-level hook
        thread::spawn(move || unsafe {
            let h_instance = GetModuleHandleW(null_mut());
            loop {
                let hook = SetWindowsHookExW(
                    WH_KEYBOARD_LL,
                    Some(low_level_keyboard_proc),
                    h_instance,
                    0,
                );

                if hook.is_null() {
                    eprintln!("[Windows Dial] Failed to install WH_KEYBOARD_LL low-level hook. Retrying in 2s...");
                    thread::sleep(Duration::from_millis(2000));
                    continue;
                }

                HOOK_HANDLE.store(hook as usize, Ordering::SeqCst);
                println!("[Windows Dial] Low-level keyboard & AVRCP hook installed successfully.");

                // Set a 2000ms timer so GetMessageW unblocks periodically to keep the thread responsive and healthy
                SetTimer(null_mut(), 0, 2000, None);

                let mut msg: MSG = std::mem::zeroed();
                while GetMessageW(&mut msg, null_mut(), 0, 0) > 0 {
                    TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }

                UnhookWindowsHookEx(hook);
                HOOK_HANDLE.store(0, Ordering::SeqCst);
                println!("[Windows Dial] Low-level keyboard hook uninstalled or message loop exited. Re-installing hook...");
                thread::sleep(Duration::from_millis(1000));
            }
        });
    }
}
