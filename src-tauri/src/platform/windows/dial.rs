use crate::platform::traits::DialBackend;
use crate::platform::windows::ffi::*;
use std::ptr::null_mut;
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicUsize, Ordering};
use std::sync::{Arc, RwLock};
use std::thread;
use tauri::{AppHandle, Emitter};

static IS_HUD_OPEN_REF: RwLock<Option<Arc<AtomicBool>>> = RwLock::new(None);
static APP_HANDLE_REF: RwLock<Option<AppHandle>> = RwLock::new(None);
static HOOK_HANDLE: AtomicUsize = AtomicUsize::new(0);

#[derive(Clone, serde::Serialize)]
struct RotatePayload {
    delta: i32,
}

unsafe extern "system" fn low_level_keyboard_proc(
    n_code: i32,
    w_param: WPARAM,
    l_param: LPARAM,
) -> LRESULT {
    if n_code >= 0 && (w_param == WM_KEYDOWN as usize || w_param == WM_SYSKEYDOWN as usize) {
        let kbd = *(l_param as *const KBDLLHOOKSTRUCT);
        let vk = kbd.vkCode as u16;

        let is_hud_open = if let Ok(guard) = IS_HUD_OPEN_REF.read() {
            guard.as_ref().map(|arc| arc.load(Ordering::SeqCst)).unwrap_or(false)
        } else {
            false
        };

        if is_hud_open {
            match vk {
                VK_VOLUME_UP => {
                    println!("[Windows Dial] Intercepted Volume Up -> dial:rotate (+1)");
                    if let Ok(guard) = APP_HANDLE_REF.read() {
                        if let Some(ref app) = *guard {
                            let _ = app.emit("dial:rotate", RotatePayload { delta: 1 });
                        }
                    }
                    // Block volume key from changing Windows system volume
                    return 1;
                }
                VK_VOLUME_DOWN => {
                    println!("[Windows Dial] Intercepted Volume Down -> dial:rotate (-1)");
                    if let Ok(guard) = APP_HANDLE_REF.read() {
                        if let Some(ref app) = *guard {
                            let _ = app.emit("dial:rotate", RotatePayload { delta: -1 });
                        }
                    }
                    // Block volume key from changing Windows system volume
                    return 1;
                }
                VK_MEDIA_PLAY_PAUSE => {
                    println!("[Windows Dial] Intercepted Play/Pause -> dial:click");
                    if let Ok(guard) = APP_HANDLE_REF.read() {
                        if let Some(ref app) = *guard {
                            let _ = app.emit("dial:click", ());
                        }
                    }
                    // Block media key from toggling playback while selecting option
                    return 1;
                }
                _ => {}
            }
        }
    }

    let hhook = HOOK_HANDLE.load(Ordering::SeqCst) as HHOOK;
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
        _locked_vol: Arc<AtomicI32>,
    ) {
        if let Ok(mut guard) = IS_HUD_OPEN_REF.write() {
            *guard = Some(is_hud_open);
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
