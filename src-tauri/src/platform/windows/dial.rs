use crate::platform::traits::{DialBackend, PlatformProvider};
use crate::platform::windows::ffi::*;
use crate::AppState;
use std::ptr::null_mut;
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicUsize, Ordering};
use std::sync::{Arc, RwLock};
use std::thread;
use tauri::{AppHandle, Emitter, Manager};

static IS_HUD_OPEN_REF: RwLock<Option<Arc<AtomicBool>>> = RwLock::new(None);
static LOCKED_VOL_REF: RwLock<Option<Arc<AtomicI32>>> = RwLock::new(None);
static IS_SCROLL_MODE_REF: RwLock<Option<Arc<AtomicBool>>> = RwLock::new(None);
static APP_HANDLE_REF: RwLock<Option<AppHandle>> = RwLock::new(None);
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

    if n_code >= 0 && (w_param == WM_KEYDOWN as usize || w_param == WM_SYSKEYDOWN as usize) {
        let kbd = *(l_param as *const KBDLLHOOKSTRUCT);

        // Passthrough if the input was synthesized by Dial Control itself
        if kbd.dwExtraInfo == SYNTHETIC_INPUT_TAG {
            return CallNextHookEx(hhook, n_code, w_param, l_param);
        }

        let vk = kbd.vkCode as u16;

        let is_hud_open = if let Ok(guard) = IS_HUD_OPEN_REF.read() {
            guard.as_ref().map(|arc| arc.load(Ordering::SeqCst)).unwrap_or(false)
        } else {
            false
        };

        let is_scroll_mode = if let Ok(guard) = IS_SCROLL_MODE_REF.read() {
            guard.as_ref().map(|arc| arc.load(Ordering::SeqCst)).unwrap_or(false)
        } else {
            false
        };

        match vk {
            VK_MEDIA_PLAY_PAUSE | VK_PAUSE | VK_MEDIA_STOP => {
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
                    if let Ok(guard) = IS_SCROLL_MODE_REF.read() {
                        if let Some(ref arc) = *guard {
                            arc.store(false, Ordering::SeqCst);
                        }
                    }
                    if let Ok(guard) = LOCKED_VOL_REF.read() {
                        if let Some(ref arc) = *guard {
                            arc.store(-1, Ordering::SeqCst);
                        }
                    }
                    if let Ok(guard) = IS_HUD_OPEN_REF.read() {
                        if let Some(ref arc) = *guard {
                            arc.store(false, Ordering::SeqCst);
                        }
                    }
                    if let Ok(guard) = APP_HANDLE_REF.read() {
                        if let Some(ref app) = *guard {
                            let app = app.clone();
                            thread::spawn(move || {
                                let state: tauri::State<AppState> = app.state();
                                crate::do_stop_scroll_mode(&app, &state);
                            });
                        }
                    }
                    return 1;
                }

                if !is_hud_open {
                    // 1. Immediately flip atomic flag so consecutive inputs know HUD is active
                    if let Ok(is_open_guard) = IS_HUD_OPEN_REF.read() {
                        if let Some(ref is_open) = *is_open_guard {
                            is_open.store(true, Ordering::SeqCst);
                        }
                    }

                    // 2. Offload GUI display to worker thread so the low-level hook returns in <0.01ms,
                    // preventing Windows from timing out and silently removing the hook!
                    if let Ok(guard) = APP_HANDLE_REF.read() {
                        if let Some(ref app) = *guard {
                            let app = app.clone();
                            thread::spawn(move || {
                                println!("[Windows Dial] Intercepted Earbuds Dial Click -> Opening HUD");
                                let state: tauri::State<AppState> = app.state();
                                crate::do_show_hud(&app, &state);
                            });
                        }
                    }
                    // Block OS from toggling audio playback
                    return 1;
                } else {
                    println!("[Windows Dial] Intercepted Earbuds Dial Click -> Confirm selection");
                    if let Ok(guard) = APP_HANDLE_REF.read() {
                        if let Some(ref app) = *guard {
                            let app = app.clone();
                            thread::spawn(move || {
                                let _ = app.emit("dial:click", ());
                                if let Some(window) = app.get_webview_window("hud") {
                                    let _ = window.emit("dial:click", ());
                                }
                            });
                        }
                    }
                    // Block OS from toggling audio playback
                    return 1;
                }
            }
            VK_VOLUME_UP | VK_VOLUME_DOWN if (is_hud_open || is_scroll_mode) => {
                let dir = if vk == VK_VOLUME_UP { 1 } else { -1 };

                if is_scroll_mode {
                    // Clockwise rotation (dir = 1) -> Scroll DOWN (-1)
                    // Counterclockwise rotation (dir = -1) -> Scroll UP (1)
                    let scroll_delta = if dir > 0 { -1 } else { 1 };
                    println!("[Windows Dial] Scroll Mode Emulation -> dir: {}, delta: {}", dir, scroll_delta);
                    if let Ok(guard) = APP_HANDLE_REF.read() {
                        if let Some(ref app) = *guard {
                            let app = app.clone();
                            thread::spawn(move || {
                                let state: tauri::State<AppState> = app.state();
                                state.platform.input().emit_scroll(scroll_delta);
                                let _ = app.emit("scroll:tick", serde_json::json!({ "direction": dir }));
                            });
                        }
                    }
                    return 1;
                }
                let dir = if vk == VK_VOLUME_UP { 1 } else { -1 };
                println!("[Windows Dial] Intercepted Dial Rotate -> ({})", dir);

                let locked_val = if let Ok(guard) = LOCKED_VOL_REF.read() {
                    guard.as_ref().map(|arc| arc.load(Ordering::SeqCst)).unwrap_or(-1)
                } else {
                    -1
                };

                if let Ok(guard) = APP_HANDLE_REF.read() {
                    if let Some(ref app) = *guard {
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
                }
                // Block volume key from default OS beep/volume popup while navigating
                return 1;
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
        if let Ok(mut guard) = IS_HUD_OPEN_REF.write() {
            *guard = Some(is_hud_open);
        }
        if let Ok(mut guard) = LOCKED_VOL_REF.write() {
            *guard = Some(locked_vol);
        }
        if let Ok(mut guard) = IS_SCROLL_MODE_REF.write() {
            *guard = Some(is_scroll_mode);
        }
        if let Ok(mut guard) = APP_HANDLE_REF.write() {
            *guard = Some(app);
        }

        // Spawn a dedicated thread with a Win32 message pump for the low-level hook
        thread::spawn(move || unsafe {
            let h_instance = GetModuleHandleW(null_mut());
            let hook = SetWindowsHookExW(
                WH_KEYBOARD_LL,
                Some(low_level_keyboard_proc),
                h_instance,
                0,
            );

            if hook.is_null() {
                eprintln!("[Windows Dial] Failed to install WH_KEYBOARD_LL low-level hook.");
                return;
            }

            HOOK_HANDLE.store(hook as usize, Ordering::SeqCst);
            println!("[Windows Dial] Low-level keyboard & AVRCP hook installed successfully.");

            let mut msg: MSG = std::mem::zeroed();
            while GetMessageW(&mut msg, null_mut(), 0, 0) > 0 {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }

            UnhookWindowsHookEx(hook);
            println!("[Windows Dial] Low-level keyboard hook uninstalled.");
        });
    }
}
