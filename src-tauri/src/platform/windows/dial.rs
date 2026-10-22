use crate::platform::traits::{DialBackend, SystemAudio};
use crate::platform::windows::ffi::*;
use std::ptr::null_mut;
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicUsize, Ordering};
use std::sync::{Arc, RwLock};
use std::thread;
use tauri::{AppHandle, Emitter, Manager};

static IS_HUD_OPEN_REF: RwLock<Option<Arc<AtomicBool>>> = RwLock::new(None);
static LOCKED_VOL_REF: RwLock<Option<Arc<AtomicI32>>> = RwLock::new(None);
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

        match vk {
            VK_MEDIA_PLAY_PAUSE => {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as usize;

                let last = LAST_DIAL_CLICK_TIME.load(Ordering::SeqCst);
                if now.saturating_sub(last) < 400 {
                    // Hardware contact debounce: block repeat event
                    return 1;
                }
                LAST_DIAL_CLICK_TIME.store(now, Ordering::SeqCst);

                if !is_hud_open {
                    // Earbuds dial clicked while HUD is closed -> OPEN THE RADIAL HUD!
                    println!("[Windows Dial] Intercepted Earbuds Dial Click -> Opening HUD");
                    if let Ok(guard) = APP_HANDLE_REF.read() {
                        if let Some(ref app) = *guard {
                            if let Ok(is_open_guard) = IS_HUD_OPEN_REF.read() {
                                if let Some(ref is_open) = *is_open_guard {
                                    is_open.store(true, Ordering::SeqCst);
                                }
                            }
                            if let Ok(locked_guard) = LOCKED_VOL_REF.read() {
                                if let Some(ref locked) = *locked_guard {
                                    locked.store(50, Ordering::SeqCst);
                                }
                            }

                            if let Some(window) = app.get_webview_window("hud") {
                                let _ = window.unminimize();
                                let _ = window.show();
                                let _ = window.set_focus();
                                let _ = window.set_always_on_top(true);
                                let _ = window.emit("hud:visibility", true);
                            }
                        }
                    }
                    // Block OS from toggling audio playback
                    return 1;
                } else {
                    // Earbuds dial clicked while HUD is open -> CONFIRM SELECTED ACTION!
                    println!("[Windows Dial] Intercepted Earbuds Dial Click -> Confirm selection");
                    if let Ok(guard) = APP_HANDLE_REF.read() {
                        if let Some(ref app) = *guard {
                            let _ = app.emit("dial:click", ());
                        }
                    }
                    // Block OS from toggling audio playback
                    return 1;
                }
            }
            VK_VOLUME_UP | VK_VOLUME_DOWN if is_hud_open => {
                let dir = if vk == VK_VOLUME_UP { 1 } else { -1 };
                println!("[Windows Dial] Intercepted Dial Rotate -> ({})", dir);

                let locked_val = if let Ok(guard) = LOCKED_VOL_REF.read() {
                    guard.as_ref().map(|arc| arc.load(Ordering::SeqCst)).unwrap_or(0)
                } else {
                    0
                };

                let audio = crate::platform::windows::audio::WindowsAudio::new();
                let current_vol = audio.get_volume();

                let new_vol = if locked_val == 0 {
                    audio.set_volume((current_vol + dir * 2).clamp(0, 100))
                } else {
                    current_vol
                };

                if let Ok(guard) = APP_HANDLE_REF.read() {
                    if let Some(ref app) = *guard {
                        let _ = app.emit(
                            "dial:rotate",
                            RotatePayload {
                                direction: dir,
                                delta: dir,
                                volume: new_vol,
                                volume_pct: new_vol,
                            },
                        );
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
    ) {
        if let Ok(mut guard) = IS_HUD_OPEN_REF.write() {
            *guard = Some(is_hud_open);
        }
        if let Ok(mut guard) = LOCKED_VOL_REF.write() {
            *guard = Some(locked_vol);
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
