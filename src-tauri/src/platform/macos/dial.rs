use crate::platform::traits::{DialBackend, PlatformProvider};
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicUsize, Ordering};
use std::sync::{Arc, OnceLock};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter, Manager};

#[cfg(target_os = "macos")]
use super::ffi::*;

static IS_HUD_OPEN_REF: OnceLock<Arc<AtomicBool>> = OnceLock::new();
static LOCKED_VOL_REF: OnceLock<Arc<AtomicI32>> = OnceLock::new();
static IS_SCROLL_MODE_REF: OnceLock<Arc<AtomicBool>> = OnceLock::new();
static APP_HANDLE_REF: OnceLock<AppHandle> = OnceLock::new();
static LAST_CLICK_TIME_MS: AtomicUsize = AtomicUsize::new(0);
static TAP_PORT_REF: AtomicUsize = AtomicUsize::new(0);

pub struct MacDial;

impl MacDial {
    pub fn new() -> Self {
        Self
    }

    #[cfg(target_os = "macos")]
    unsafe extern "C" fn event_tap_callback(
        _proxy: CGEventTapProxy,
        event_type: CGEventType,
        event: CGEventRef,
        _user_info: *mut std::ffi::c_void,
    ) -> CGEventRef {
        if event_type == kCGEventTapDisabledByTimeout {
            let port_addr = TAP_PORT_REF.load(Ordering::SeqCst);
            if port_addr != 0 {
                CGEventTapEnable(port_addr as CFMachPortRef, true);
                println!("[macOS Dial] Auto re-enabled CGEventTap after timeout.");
            }
            return event;
        }

        if event_type == kCGEventTapDisabledByUserInput {
            return event;
        }

        if event_type == NX_SYSDEFINED {
            let mut subtype: i16 = 0;
            let mut data1: isize = 0;

            // 1. Try public AppKit [NSEvent eventWithCGEvent:]
            let cls_nsevent = objc_getClass(b"NSEvent\0".as_ptr() as *const _);
            if !cls_nsevent.is_null() {
                let sel_event_with_cg =
                    sel_registerName(b"eventWithCGEvent:\0".as_ptr() as *const _);
                let sel_subtype = sel_registerName(b"subtype\0".as_ptr() as *const _);
                let sel_data1 = sel_registerName(b"data1\0".as_ptr() as *const _);

                type MsgSendCG = unsafe extern "C" fn(
                    *mut std::ffi::c_void,
                    *mut std::ffi::c_void,
                    CGEventRef,
                ) -> *mut std::ffi::c_void;
                type MsgSendSubtype =
                    unsafe extern "C" fn(*mut std::ffi::c_void, *mut std::ffi::c_void) -> i16;
                type MsgSendData1 =
                    unsafe extern "C" fn(*mut std::ffi::c_void, *mut std::ffi::c_void) -> isize;

                let msg_cg: MsgSendCG = std::mem::transmute(objc_msgSend as *const ());
                let msg_sub: MsgSendSubtype = std::mem::transmute(objc_msgSend as *const ());
                let msg_d1: MsgSendData1 = std::mem::transmute(objc_msgSend as *const ());

                let ns_event = msg_cg(cls_nsevent, sel_event_with_cg, event);
                if !ns_event.is_null() {
                    subtype = msg_sub(ns_event, sel_subtype);
                    data1 = msg_d1(ns_event, sel_data1);
                }
            }

            // Fallback: raw CoreGraphics field 54 or 0x95
            if subtype == 0 && data1 == 0 {
                let raw_data1 = CGEventGetIntegerValueField(event, 54);
                subtype = (raw_data1 & 0xFF) as i16;
                data1 = raw_data1 as isize;
            }

            if subtype == NX_SUBTYPE_AUX_CONTROL_BUTTONS {
                let key_type = ((data1 >> 16) & 0xFFFF) as u32;
                let key_flags = (data1 & 0x0000FF00) >> 8;
                let is_key_down = (key_flags & 0xFF) == 0x0A || (key_flags & 0x01) != 0;

                let is_scroll = IS_SCROLL_MODE_REF
                    .get()
                    .map(|arc| arc.load(Ordering::SeqCst))
                    .unwrap_or(false);

                let is_hud_open = IS_HUD_OPEN_REF
                    .get()
                    .map(|arc| arc.load(Ordering::SeqCst))
                    .unwrap_or(false);

                let locked_val = LOCKED_VOL_REF
                    .get()
                    .map(|arc| arc.load(Ordering::SeqCst))
                    .unwrap_or(-1);

                // 1. Center Click / Media Play-Pause Key
                if key_type == NX_KEYTYPE_PLAY {
                    if is_key_down {
                        let now = SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_millis() as usize;

                        let last = LAST_CLICK_TIME_MS.load(Ordering::SeqCst);
                        if now.saturating_sub(last) < 250 {
                            return std::ptr::null_mut();
                        }
                        LAST_CLICK_TIME_MS.store(now, Ordering::SeqCst);

                        if is_scroll {
                            println!(">> [macOS NATIVE DIAL CLICK] -> Exiting Scroll Mode");
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
                                let app_clone = app.clone();
                                thread::spawn(move || {
                                    let state: tauri::State<crate::AppState> = app_clone.state();
                                    crate::do_stop_scroll_mode(&app_clone, &state);
                                });
                            }
                            return std::ptr::null_mut();
                        }

                        if !is_hud_open {
                            println!(">> [macOS NATIVE DIAL CLICK] -> Opening HUD");
                            // Synchronously lock volume and set open flag so incoming rotation ticks
                            // are immediately intercepted while do_show_hud starts up!
                            if let Some(arc) = IS_HUD_OPEN_REF.get() {
                                arc.store(true, Ordering::SeqCst);
                            }
                            if let Some(arc) = LOCKED_VOL_REF.get() {
                                arc.store(50, Ordering::SeqCst);
                            }

                            if let Some(app) = APP_HANDLE_REF.get() {
                                let app_clone = app.clone();
                                thread::spawn(move || {
                                    let state: tauri::State<crate::AppState> = app_clone.state();
                                    crate::do_show_hud(&app_clone, &state);
                                });
                            }
                            return std::ptr::null_mut();
                        } else {
                            println!(">> [macOS NATIVE DIAL CLICK] -> Dispatched action select to HUD");
                            if let Some(app) = APP_HANDLE_REF.get() {
                                let app_clone = app.clone();
                                thread::spawn(move || {
                                    let _ = app_clone.emit("dial:click", ());
                                    if let Some(window) = app_clone.get_webview_window("hud") {
                                        let _ = window.emit("dial:click", ());
                                    }
                                });
                            }
                            return std::ptr::null_mut();
                        }
                    }

                    // Return null to drop the event so macOS Music/Spotify doesn't play/pause
                    return std::ptr::null_mut();
                }

                // 2. Rotary Dial Rotation (Volume Up = CW, Volume Down = CCW)
                if key_type == NX_KEYTYPE_SOUND_UP || key_type == NX_KEYTYPE_SOUND_DOWN {
                    let dir = if key_type == NX_KEYTYPE_SOUND_UP { 1 } else { -1 };

                    if is_scroll {
                        if is_key_down {
                            let scroll_delta = if dir > 0 { -1 } else { 1 };
                            if let Some(app) = APP_HANDLE_REF.get() {
                                let app_clone = app.clone();
                                thread::spawn(move || {
                                    let state: tauri::State<crate::AppState> = app_clone.state();
                                    state.platform.input().emit_scroll(scroll_delta);
                                    let _ = app_clone.emit("scroll:tick", serde_json::json!({ "direction": dir }));
                                });
                            }
                        }
                        // Drop event (both down and up): prevent OS volume change
                        return std::ptr::null_mut();
                    }

                    if is_hud_open {
                        if is_key_down {
                            if let Some(app) = APP_HANDLE_REF.get() {
                                let app_clone = app.clone();
                                thread::spawn(move || {
                                    let state: tauri::State<crate::AppState> = app_clone.state();

                                    let target_vol = if locked_val < 0 {
                                        let current_vol = state.platform.audio().get_volume();
                                        state.platform.audio().set_volume((current_vol + dir * 2).clamp(0, 100))
                                    } else {
                                        locked_val
                                    };

                                    let payload = serde_json::json!({
                                        "direction": dir,
                                        "delta": dir,
                                        "volume": target_vol,
                                        "volume_pct": target_vol,
                                    });
                                    let _ = app_clone.emit("dial:rotate", payload.clone());
                                    if let Some(window) = app_clone.get_webview_window("hud") {
                                        let _ = window.emit("dial:rotate", payload);
                                    }
                                });
                            }
                        }

                        // Always drop volume event while HUD is open so macOS doesn't alter system volume or show volume HUD
                        return std::ptr::null_mut();
                    }
                }
            }
        }

        event
    }
}

impl DialBackend for MacDial {
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

        #[cfg(target_os = "macos")]
        thread::spawn(move || unsafe {
            let event_mask: CGEventMask = 1 << NX_SYSDEFINED;
            let mut port: CFMachPortRef = std::ptr::null_mut();

            // Try creating the event tap, retrying every 2 seconds if accessibility permissions are pending
            for _ in 0..60 {
                port = CGEventTapCreate(
                    kCGHIDEventTap,
                    kCGHeadInsertEventTap,
                    kCGEventTapOptionDefault,
                    event_mask,
                    Self::event_tap_callback,
                    std::ptr::null_mut(),
                );

                if !port.is_null() {
                    break;
                }

                eprintln!("[!] macOS CGEventTap requires Accessibility permission. Retrying in 2s...");
                thread::sleep(Duration::from_secs(2));
            }

            if port.is_null() {
                eprintln!("[!] Could not create macOS CGEventTap after retries. Please enable Accessibility in System Settings -> Privacy & Security.");
                return;
            }

            TAP_PORT_REF.store(port as usize, Ordering::SeqCst);

            println!("[+] macOS CGEventTap for rotary dial & media keys initialized successfully.");

            let run_loop_source = CFMachPortCreateRunLoopSource(std::ptr::null(), port, 0);
            let run_loop = CFRunLoopGetCurrent();

            CFRunLoopAddSource(run_loop, run_loop_source, kCFRunLoopCommonModes);
            CGEventTapEnable(port, true);

            CFRunLoopRun();
        });

        #[cfg(not(target_os = "macos"))]
        {
            println!("[macOS Mock Dial] Listeners registered");
        }
    }
}
