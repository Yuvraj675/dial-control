use crate::platform::traits::{DialBackend, PlatformProvider};
use crate::platform::windows::ffi::*;
use crate::AppState;
use std::ffi::c_void;
use std::ptr::null_mut;
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU32, AtomicUsize, Ordering};
use std::sync::{Arc, RwLock};
use std::thread;
use tauri::{AppHandle, Emitter, Manager};

static IS_HUD_OPEN_REF: RwLock<Option<Arc<AtomicBool>>> = RwLock::new(None);
static LOCKED_VOL_REF: RwLock<Option<Arc<AtomicI32>>> = RwLock::new(None);
static IS_SCROLL_MODE_REF: RwLock<Option<Arc<AtomicBool>>> = RwLock::new(None);
static APP_HANDLE_REF: RwLock<Option<AppHandle>> = RwLock::new(None);
static HOOK_HANDLE: AtomicUsize = AtomicUsize::new(0);
static FLYOUT_HOOK_HANDLE: AtomicUsize = AtomicUsize::new(0);
static LAST_DIAL_CLICK_TIME: AtomicUsize = AtomicUsize::new(0);
static LAST_AUDIO_NOTIFY_TIME: AtomicUsize = AtomicUsize::new(0);
static HAS_ROTATED: AtomicBool = AtomicBool::new(false);

static LAST_AUDIO_LEVEL_BITS: AtomicU32 = AtomicU32::new(0);
static LAST_SET_LEVEL_BITS: AtomicU32 = AtomicU32::new(0);
static LAST_SET_TIME: AtomicUsize = AtomicUsize::new(0);

#[inline]
fn get_last_audio_level() -> f32 {
    f32::from_bits(LAST_AUDIO_LEVEL_BITS.load(Ordering::SeqCst))
}

#[inline]
fn set_last_audio_level(lvl: f32) {
    LAST_AUDIO_LEVEL_BITS.store(lvl.to_bits(), Ordering::SeqCst);
}

#[inline]
fn get_last_set_level() -> f32 {
    f32::from_bits(LAST_SET_LEVEL_BITS.load(Ordering::SeqCst))
}

#[inline]
fn set_last_set_level(lvl: f32) {
    LAST_SET_LEVEL_BITS.store(lvl.to_bits(), Ordering::SeqCst);
}

pub fn init_hud_rotation(curr_vol: i32) {
    HAS_ROTATED.store(false, Ordering::SeqCst);
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as usize;

    // Center volume around 50% if near boundaries to guarantee headroom for turns in both directions
    let target = if curr_vol < 25 || curr_vol > 75 {
        let _ = crate::platform::windows::audio::set_hardware_volume_with_context(
            50,
            &DIAL_CONTROL_EVENT_CONTEXT,
        );
        0.50
    } else {
        curr_vol as f32 / 100.0
    };

    set_last_set_level(target);
    LAST_SET_TIME.store(now, Ordering::SeqCst);
    set_last_audio_level(target);
    LAST_AUDIO_NOTIFY_TIME.store(now, Ordering::SeqCst);
}

pub fn cleanup_hud_rotation() {
    HAS_ROTATED.store(false, Ordering::SeqCst);
}

#[derive(Clone, serde::Serialize)]
struct RotatePayload {
    direction: i32,
    delta: i32,
    volume: i32,
    volume_pct: i32,
}

// ── Low-Level Keyboard Hook (WH_KEYBOARD_LL) ─────────────────────────────────
// Catches media play/pause keys from the dial case button, next/prev track gestures,
// and hardware keyboard volume keys.
unsafe extern "system" fn low_level_keyboard_proc(
    n_code: i32,
    w_param: WPARAM,
    l_param: LPARAM,
) -> LRESULT {
    let hhook = HOOK_HANDLE.load(Ordering::SeqCst) as HHOOK;

    if n_code >= 0 {
        let is_key_down = w_param == WM_KEYDOWN as usize || w_param == WM_SYSKEYDOWN as usize;
        let is_key_up = w_param == WM_KEYUP as usize || w_param == WM_SYSKEYUP as usize;

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
                // ALWAYS consume KeyUp so media players (Spotify, YouTube, Chrome)
                // NEVER receive an unconsumed key release when clicking the physical dial!
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
                        // Hardware contact debounce: block repeat burst
                        return 1;
                    }
                    LAST_DIAL_CLICK_TIME.store(now, Ordering::SeqCst);

                    if is_scroll_mode {
                        println!("[Windows Dial] Intercepted Dial Click -> Exiting Scroll Mode");
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
                        // 1. Immediately flip atomic flags so subsequent events know HUD is opening
                        if let Ok(is_open_guard) = IS_HUD_OPEN_REF.read() {
                            if let Some(ref is_open) = *is_open_guard {
                                is_open.store(true, Ordering::SeqCst);
                            }
                        }
                        HAS_ROTATED.store(false, Ordering::SeqCst);

                        // 2. Grant foreground activation permission to our process
                        AllowSetForegroundWindow(GetCurrentProcessId());

                        // 3. Offload GUI display to worker thread so the low-level hook returns in <0.01ms
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
                        // HUD is open:
                        let has_rotated = HAS_ROTATED.load(Ordering::SeqCst);
                        if !has_rotated {
                            // User clicked to open, and clicked again without rotating -> TOGGLE / CLOSE HUD!
                            println!("[Windows Dial] Dial Click without rotation -> Toggling HUD closed");
                            if let Ok(guard) = APP_HANDLE_REF.read() {
                                if let Some(ref app) = *guard {
                                    let app = app.clone();
                                    thread::spawn(move || {
                                        let state: tauri::State<AppState> = app.state();
                                        crate::do_hide_hud(&app, &state);
                                    });
                                }
                            }
                            return 1;
                        } else {
                            // User rotated dial to highlight an action -> Confirm / Execute selection!
                            println!("[Windows Dial] Dial Click with rotation -> Confirm selection");
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
                            return 1;
                        }
                    }
                }
            }
            VK_MEDIA_NEXT_TRACK | VK_MEDIA_PREV_TRACK if is_hud_open || is_scroll_mode => {
                // Orbit-style gesture enhancement:
                // Double-click (Next Track) or Triple-click (Prev Track) while HUD is open
                // immediately cancels/dismisses HUD without executing any tool, and swallows
                // the keys so background media players (Spotify) do NOT skip tracks!
                if is_key_down {
                    println!("[Windows Dial] Intercepted Next/Prev Track while HUD open -> Cancelling / Dismissing HUD");
                    if let Ok(guard) = APP_HANDLE_REF.read() {
                        if let Some(ref app) = *guard {
                            let app = app.clone();
                            thread::spawn(move || {
                                let state: tauri::State<AppState> = app.state();
                                if state.is_scroll_mode.load(Ordering::SeqCst) {
                                    crate::do_stop_scroll_mode(&app, &state);
                                } else {
                                    crate::do_hide_hud(&app, &state);
                                }
                            });
                        }
                    }
                }
                return 1;
            }
            VK_VOLUME_UP | VK_VOLUME_DOWN if (is_hud_open || is_scroll_mode) => {
                if is_key_up {
                    return 1;
                }
                if is_key_down {
                    let dir = if vk == VK_VOLUME_UP { 1 } else { -1 };
                    HAS_ROTATED.store(true, Ordering::SeqCst);

                    if is_scroll_mode {
                        let scroll_delta = if dir > 0 { -1 } else { 1 };
                        println!("[Windows Dial] Keyboard Scroll Mode -> dir: {}, delta: {}", dir, scroll_delta);
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

                    println!("[Windows Dial] Intercepted Keyboard Volume -> ({})", dir);

                    let locked_val = if let Ok(guard) = LOCKED_VOL_REF.read() {
                        guard.as_ref().map(|arc| arc.load(Ordering::SeqCst)).unwrap_or(-1)
                    } else {
                        -1
                    };

                    if let Ok(guard) = APP_HANDLE_REF.read() {
                        if let Some(ref app) = *guard {
                            let app = app.clone();
                            thread::spawn(move || {
                                let payload = RotatePayload {
                                    direction: dir,
                                    delta: dir,
                                    volume: if locked_val >= 0 { locked_val } else { 50 },
                                    volume_pct: if locked_val >= 0 { locked_val } else { 50 },
                                };
                                let _ = app.emit("dial:rotate", payload.clone());
                                if let Some(window) = app.get_webview_window("hud") {
                                    let _ = window.emit("dial:rotate", payload);
                                }
                            });
                        }
                    }
                    return 1;
                }
            }
            _ => {}
        }
    }

    CallNextHookEx(hhook, n_code, w_param, l_param)
}

// ── Windows 11 Volume Flyout Suppressor ───────────────────────────────────────
// Windows 11 shows its volume bar pill (XamlExplorerHostIslandWindow) on every
// endpoint volume change. Dial Control renders its own HUD, so the OS flyout is noise.
// Out-of-context WinEvent hook suppresses the OS flyout pill during dial interaction.
unsafe extern "system" fn flyout_win_event_proc(
    _hook: HWINEVENTHOOK,
    event: DWORD,
    hwnd: HWND,
    id_object: i32,
    _id_child: i32,
    _thread: DWORD,
    _time: DWORD,
) {
    if event != EVENT_OBJECT_SHOW || id_object != 0 || hwnd.is_null() {
        return;
    }

    let is_hud_open = if let Ok(guard) = IS_HUD_OPEN_REF.read() {
        guard.as_ref().map(|arc| arc.load(Ordering::SeqCst)).unwrap_or(false)
    } else {
        false
    };

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as usize;
    let last_activity = LAST_AUDIO_NOTIFY_TIME.load(Ordering::SeqCst);
    let dial_near = now.saturating_sub(last_activity) < 800;

    if !is_hud_open && !dial_near {
        return;
    }

    let mut class_name = [0u16; 64];
    let len = GetClassNameW(hwnd, class_name.as_mut_ptr(), 64);
    if len > 0 {
        let name = String::from_utf16_lossy(&class_name[..len as usize]);
        if name == "XamlExplorerHostIslandWindow" {
            let mut r: RECT = std::mem::zeroed();
            if GetWindowRect(hwnd, &mut r) != 0 {
                let width = r.right - r.left;
                let height = r.bottom - r.top;
                // Only suppress the small volume pill, never full-size islands (Start, Widgets)
                if width > 0 && width <= 700 && height > 0 && height <= 200 {
                    ShowWindow(hwnd, 0); // SW_HIDE
                    println!("[Windows Dial] Suppressed Windows 11 volume flyout pill");
                }
            }
        }
    }
}

fn run_flyout_suppressor() {
    thread::spawn(move || unsafe {
        let hook = SetWinEventHook(
            EVENT_OBJECT_SHOW,
            EVENT_OBJECT_SHOW,
            null_mut(),
            Some(flyout_win_event_proc),
            0,
            0,
            WINEVENT_OUTOFCONTEXT,
        );

        if hook.is_null() {
            eprintln!("[Windows Dial] Warning: Could not install WinEvent hook for volume flyout suppression.");
            return;
        }

        FLYOUT_HOOK_HANDLE.store(hook as usize, Ordering::SeqCst);
        println!("[Windows Dial] Windows volume flyout suppression hook active.");

        let mut msg: MSG = std::mem::zeroed();
        while GetMessageW(&mut msg, null_mut(), 0, 0) > 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }

        UnhookWinEvent(hook);
        println!("[Windows Dial] Volume flyout hook uninstalled.");
    });
}

// ── Core Audio Endpoint Volume Callback (IAudioEndpointVolumeCallback) ───────
// Intercepts Bluetooth AVRCP Absolute Volume rotary turns from the CMF Buds Pro 2 dial.
#[repr(C)]
struct VolumeCallback {
    lp_vtbl: *const IAudioEndpointVolumeCallbackVtbl,
}

unsafe impl Sync for VolumeCallback {}
unsafe impl Send for VolumeCallback {}

unsafe extern "system" fn callback_query_interface(
    this: *mut c_void,
    riid: *const GUID,
    ppv: *mut *mut c_void,
) -> HRESULT {
    if ppv.is_null() {
        return -2147467261; // E_POINTER
    }
    let guid = *riid;
    // Match IUnknown (00000000-0000-0000-C000-000000000046) or IAudioEndpointVolumeCallback (657804FA-D6AD-4496-8A60-352752AF4F89)
    if (guid.data1 == 0 && guid.data2 == 0 && guid.data3 == 0)
        || (guid.data1 == 0x657804FA && guid.data2 == 0xD6AD && guid.data3 == 0x4496)
    {
        *ppv = this;
        return 0; // S_OK
    }
    *ppv = null_mut();
    -2147467262 // E_NOINTERFACE
}

unsafe extern "system" fn callback_add_ref(_this: *mut c_void) -> u32 {
    1
}

unsafe extern "system" fn callback_release(_this: *mut c_void) -> u32 {
    1
}

unsafe extern "system" fn callback_on_notify(
    _this: *mut c_void,
    p_notify: *mut AUDIO_VOLUME_NOTIFICATION_DATA,
) -> HRESULT {
    if p_notify.is_null() {
        return 0;
    }
    let data = *p_notify;
    let new_vol = data.fMasterVolume;

    // 1. Direct Context Echo Filter: Dial Control's own set carried DIAL_CONTROL_EVENT_CONTEXT
    if is_same_guid(&data.guidEventContext, &DIAL_CONTROL_EVENT_CONTEXT) {
        set_last_audio_level(new_vol);
        return 0;
    }

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as usize;

    // 2. Windows Duplicate Notification Echo Filter (discovered by Orbit research):
    // Windows sends a second copy of our own set ~1ms later with GUID_NULL.
    let last_set = get_last_set_level();
    let last_set_t = LAST_SET_TIME.load(Ordering::SeqCst);
    if (new_vol - last_set).abs() < 0.015 && now.saturating_sub(last_set_t) < 800 {
        set_last_audio_level(new_vol);
        return 0;
    }

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

    if !is_hud_open && !is_scroll_mode {
        set_last_audio_level(new_vol);
        return 0;
    }

    let prev = get_last_audio_level();
    let d = new_vol - prev;
    if d.abs() < 0.02 {
        // Less than ~1/3 of a 16-step grid step (0.0625): ignore keyboard nudge or sub-grid jitter
        return 0;
    }

    let dir = if d > 0.0 { 1 } else { -1 };
    set_last_audio_level(new_vol);
    HAS_ROTATED.store(true, Ordering::SeqCst);
    LAST_AUDIO_NOTIFY_TIME.store(now, Ordering::SeqCst);

    println!(
        "[Windows Dial] Rotary turn detected: {:.1}% (prev: {:.1}%, dir: {})",
        new_vol * 100.0,
        prev * 100.0,
        dir
    );

    // Headroom safety: if volume reaches close to 0% or 100%, recenter to 50%
    if new_vol <= 0.15 || new_vol >= 0.85 {
        println!("[Windows Dial] Volume near boundary ({:.1}%) -> Recentering to 50% for headroom", new_vol * 100.0);
        let _ = crate::platform::windows::audio::set_hardware_volume_with_context(
            50,
            &DIAL_CONTROL_EVENT_CONTEXT,
        );
        set_last_set_level(0.50);
        LAST_SET_TIME.store(now, Ordering::SeqCst);
        set_last_audio_level(0.50);
    }

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

                if is_scroll_mode {
                    let scroll_delta = if dir > 0 { -1 } else { 1 };
                    state.platform.input().emit_scroll(scroll_delta);
                    let _ = app.emit("scroll:tick", serde_json::json!({ "direction": dir }));
                } else if is_hud_open {
                    let payload = RotatePayload {
                        direction: dir,
                        delta: dir,
                        volume: if locked_val >= 0 { locked_val } else { (new_vol * 100.0).round() as i32 },
                        volume_pct: if locked_val >= 0 { locked_val } else { (new_vol * 100.0).round() as i32 },
                    };
                    let _ = app.emit("dial:rotate", payload.clone());
                    if let Some(window) = app.get_webview_window("hud") {
                        let _ = window.emit("dial:rotate", payload);
                    }
                }
            });
        }
    }

    0
}

static VOLUME_CALLBACK_VTBL: IAudioEndpointVolumeCallbackVtbl = IAudioEndpointVolumeCallbackVtbl {
    QueryInterface: callback_query_interface,
    AddRef: callback_add_ref,
    Release: callback_release,
    OnNotify: callback_on_notify,
};

static VOLUME_CALLBACK_INSTANCE: VolumeCallback = VolumeCallback {
    lp_vtbl: &VOLUME_CALLBACK_VTBL,
};

fn run_coreaudio_volume_listener() {
    thread::spawn(move || unsafe {
        let _ = CoInitializeEx(null_mut(), 0); // COINIT_MULTITHREADED

        loop {
            let mut enumerator: *mut *const IMMDeviceEnumeratorVtbl = null_mut();
            let hr = CoCreateInstance(
                &CLSID_MMDEVICE_ENUMERATOR,
                null_mut(),
                CLSCTX_ALL,
                &IID_IMMDEVICE_ENUMERATOR,
                &mut enumerator as *mut _ as *mut *mut c_void,
            );
            if hr < 0 || enumerator.is_null() {
                thread::sleep(std::time::Duration::from_millis(2000));
                continue;
            }

            let mut device: *mut *const IMMDeviceVtbl = null_mut();
            let hr = ((**enumerator).GetDefaultAudioEndpoint)(
                enumerator as *mut c_void,
                0, // eRender
                1, // eMultimedia
                &mut device as *mut _ as *mut *mut c_void,
            );
            if hr < 0 || device.is_null() {
                ((**enumerator).base.Release)(enumerator as *mut c_void);
                thread::sleep(std::time::Duration::from_millis(2000));
                continue;
            }

            let mut volume: *mut *const IAudioEndpointVolumeVtbl = null_mut();
            let hr = ((**device).Activate)(
                device as *mut c_void,
                &IID_IAUDIO_ENDPOINT_VOLUME,
                CLSCTX_ALL,
                null_mut(),
                &mut volume as *mut _ as *mut *mut c_void,
            );

            // Read device ID for tracking device switch / reconnection
            let mut bound_id = String::new();
            let mut id_ptr: *mut u16 = null_mut();
            if ((**device).GetId)(device as *mut c_void, &mut id_ptr) >= 0 && !id_ptr.is_null() {
                let mut len = 0;
                while *id_ptr.add(len) != 0 {
                    len += 1;
                }
                bound_id = String::from_utf16_lossy(std::slice::from_raw_parts(id_ptr, len));
                CoTaskMemFree(id_ptr as *mut c_void);
            }
            ((**device).base.Release)(device as *mut c_void);

            if hr < 0 || volume.is_null() {
                ((**enumerator).base.Release)(enumerator as *mut c_void);
                thread::sleep(std::time::Duration::from_millis(2000));
                continue;
            }

            let mut initial_vol: f32 = 0.50;
            let _ = ((**volume).GetMasterVolumeLevelScalar)(volume as *mut c_void, &mut initial_vol);
            set_last_audio_level(initial_vol);

            let reg_hr = ((**volume).RegisterControlChangeNotify)(
                volume as *mut c_void,
                &VOLUME_CALLBACK_INSTANCE as *const _ as *mut c_void,
            );
            if reg_hr >= 0 {
                println!("[Windows Dial] Registered Core Audio endpoint volume callback on device: {}", bound_id);
            }

            // Loop checking if default audio endpoint changes (e.g. CMF Buds connects or reconnects)
            loop {
                thread::sleep(std::time::Duration::from_millis(2000));

                let mut current_device: *mut *const IMMDeviceVtbl = null_mut();
                let hr = ((**enumerator).GetDefaultAudioEndpoint)(
                    enumerator as *mut c_void,
                    0,
                    1,
                    &mut current_device as *mut _ as *mut *mut c_void,
                );
                if hr >= 0 && !current_device.is_null() {
                    let mut current_id_ptr: *mut u16 = null_mut();
                    let mut current_id = String::new();
                    if ((**current_device).GetId)(current_device as *mut c_void, &mut current_id_ptr) >= 0 && !current_id_ptr.is_null() {
                        let mut len = 0;
                        while *current_id_ptr.add(len) != 0 {
                            len += 1;
                        }
                        current_id = String::from_utf16_lossy(std::slice::from_raw_parts(current_id_ptr, len));
                        CoTaskMemFree(current_id_ptr as *mut c_void);
                    }
                    ((**current_device).base.Release)(current_device as *mut c_void);

                    if !current_id.is_empty() && current_id != bound_id {
                        println!("[Windows Dial] Default audio endpoint changed ({} -> {}), re-binding volume callback...", bound_id, current_id);
                        break;
                    }
                }
            }

            let _ = ((**volume).UnregisterControlChangeNotify)(
                volume as *mut c_void,
                &VOLUME_CALLBACK_INSTANCE as *const _ as *mut c_void,
            );
            ((**volume).base.Release)(volume as *mut c_void);
            ((**enumerator).base.Release)(enumerator as *mut c_void);

            thread::sleep(std::time::Duration::from_millis(500));
        }
    });
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

        // 1. Start CoreAudio endpoint volume callback listener for Bluetooth AVRCP dial rotation
        run_coreaudio_volume_listener();

        // 2. Start Windows 11 volume flyout pill suppressor
        run_flyout_suppressor();

        // 3. Spawn a dedicated thread with a Win32 message pump for the low-level keyboard hook
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
