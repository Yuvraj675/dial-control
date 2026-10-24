use crate::platform::traits::{DialBackend, PlatformProvider};
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use std::sync::{Arc, RwLock};
use std::thread;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager};

#[cfg(target_os = "macos")]
use super::ffi::*;

static IS_HUD_OPEN_REF: RwLock<Option<Arc<AtomicBool>>> = RwLock::new(None);
static LOCKED_VOL_REF: RwLock<Option<Arc<AtomicI32>>> = RwLock::new(None);
static IS_SCROLL_MODE_REF: RwLock<Option<Arc<AtomicBool>>> = RwLock::new(None);
static APP_HANDLE_REF: RwLock<Option<AppHandle>> = RwLock::new(None);
static LAST_CLICK_TIME: RwLock<Option<Instant>> = RwLock::new(None);
static TAP_PORT_REF: RwLock<Option<usize>> = RwLock::new(None);

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
            if let Ok(guard) = TAP_PORT_REF.read() {
                if let Some(port_addr) = *guard {
                    CGEventTapEnable(port_addr as CFMachPortRef, true);
                    println!("[macOS Dial] Auto re-enabled CGEventTap after timeout.");
                }
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

                let is_scroll = if let Ok(guard) = IS_SCROLL_MODE_REF.read() {
                    guard.as_ref().map(|arc| arc.load(Ordering::SeqCst)).unwrap_or(false)
                } else {
                    false
                };

                let is_hud_open = if let Ok(guard) = IS_HUD_OPEN_REF.read() {
                    guard.as_ref().map(|arc| arc.load(Ordering::SeqCst)).unwrap_or(false)
                } else {
                    false
                };

                let locked_val = if let Ok(guard) = LOCKED_VOL_REF.read() {
                    guard.as_ref().map(|arc| arc.load(Ordering::SeqCst)).unwrap_or(-1)
                } else {
                    -1
                };

                // 1. Center Click / Media Play-Pause Key
                if key_type == NX_KEYTYPE_PLAY {
                    if is_key_down {
                        let now = Instant::now();
                        let mut can_click = false;
                        if let Ok(mut guard) = LAST_CLICK_TIME.write() {
                            let last = guard.unwrap_or(now - Duration::from_secs(1));
                            if now.duration_since(last) > Duration::from_millis(500) {
                                *guard = Some(now);
                                can_click = true;
                            }
                        }

                        if can_click {
                            if let Ok(guard) = APP_HANDLE_REF.read() {
                                if let Some(ref app) = *guard {
                                    let app_clone = app.clone();
                                    thread::spawn(move || {
                                        let state: tauri::State<crate::AppState> = app_clone.state();
                                        if is_scroll {
                                            println!(">> [macOS NATIVE DIAL CLICK] -> Exiting Scroll Mode");
                                            crate::do_stop_scroll_mode(&app_clone, &state);
                                        } else if !is_hud_open {
                                            println!(">> [macOS NATIVE DIAL CLICK] -> Opening HUD");
                                            crate::do_show_hud(&app_clone, &state);
                                        } else {
                                            println!(">> [macOS NATIVE DIAL CLICK] -> Dispatched action select to HUD");
                                            let _ = app_clone.emit("dial:click", ());
                                            if let Some(window) = app_clone.get_webview_window("hud") {
                                                let _ = window.emit("dial:click", ());
                                            }
                                        }
                                    });
                                }
                            }
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
                            if let Ok(guard) = APP_HANDLE_REF.read() {
                                if let Some(ref app) = *guard {
                                    let app_clone = app.clone();
                                    thread::spawn(move || {
                                        let state: tauri::State<crate::AppState> = app_clone.state();
                                        state.platform.input().emit_scroll(scroll_delta);
                                        let _ = app_clone.emit("scroll:tick", serde_json::json!({ "direction": dir }));
                                    });
                                }
                            }
                        }
                        // Drop event (both down and up): prevent OS volume change
                        return std::ptr::null_mut();
                    }

                    if is_hud_open {
                        if is_key_down {
                            if let Ok(guard) = APP_HANDLE_REF.read() {
                                if let Some(ref app) = *guard {
                                    let app_clone = app.clone();
                                    thread::spawn(move || {
                                        let state: tauri::State<crate::AppState> = app_clone.state();
                                        let current_vol = state.platform.audio().get_volume();

                                        let target_vol = if locked_val < 0 {
                                            state.platform.audio().set_volume((current_vol + dir * 2).clamp(0, 100))
                                        } else {
                                            current_vol
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
                        }

                        // If volume is locked, return null to consume key event completely!
                        if locked_val >= 0 {
                            return std::ptr::null_mut();
                        }
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

            if let Ok(mut guard) = TAP_PORT_REF.write() {
                *guard = Some(port as usize);
            }

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
