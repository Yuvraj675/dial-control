use crate::platform::dial_logic::{
    classify_click, classify_multi_press, classify_volume_change, decode_aux_key, headroom_lock, is_rotation_burst,
    now_millis, scroll_delta_for, ClickAction, GestureAction, MultiPress, RemoteCommand, VolumeAction,
};
use crate::platform::traits::{DialBackend, PlatformProvider};
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, OnceLock};
use std::thread;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};

#[cfg(target_os = "macos")]
use super::ffi::*;

static IS_HUD_OPEN_REF: OnceLock<Arc<AtomicBool>> = OnceLock::new();
static LOCKED_VOL_REF: OnceLock<Arc<AtomicI32>> = OnceLock::new();
static IS_SCROLL_MODE_REF: OnceLock<Arc<AtomicBool>> = OnceLock::new();
static APP_HANDLE_REF: OnceLock<AppHandle> = OnceLock::new();
static LAST_CLICK_TIME_MS: AtomicU64 = AtomicU64::new(0);
static LAST_GESTURE_TIME_MS: AtomicU64 = AtomicU64::new(0);
static TAP_PORT_REF: AtomicU64 = AtomicU64::new(0);

/// Volume the dial snaps back to while the HUD is open. Separate from `locked_vol`
/// (the user's volume, restored on close) because it is clamped for rotation headroom.
static ROTATION_BASELINE: AtomicI32 = AtomicI32::new(-1);
static LAST_ROTATION_MS: AtomicU64 = AtomicU64::new(0);
static BOUND_OUTPUT_DEVICE: AtomicU32 = AtomicU32::new(0);

/// Center-click debounce for hardware contact bounce.
const CLICK_DEBOUNCE_MS: u64 = 250;

fn flag(slot: &OnceLock<Arc<AtomicBool>>) -> bool {
    slot.get().map(|a| a.load(Ordering::SeqCst)).unwrap_or(false)
}

fn locked_vol() -> i32 {
    LOCKED_VOL_REF.get().map(|a| a.load(Ordering::SeqCst)).unwrap_or(-1)
}

fn with_app<F: FnOnce(AppHandle) + Send + 'static>(f: F) {
    if let Some(app) = APP_HANDLE_REF.get() {
        let app = app.clone();
        thread::spawn(move || f(app));
    }
}

/// Called from `do_show_hud`: moves the volume off 0%/100% so a CoreAudio-observed turn
/// always has room to register in both directions.
pub fn init_hud_rotation(curr_vol: i32) {
    if let Some(app) = APP_HANDLE_REF.get() {
        super::now_playing::reassert(app);
    }
    let baseline = headroom_lock(curr_vol);
    ROTATION_BASELINE.store(baseline, Ordering::SeqCst);
    if baseline != curr_vol {
        with_app(move |app| {
            let state: tauri::State<crate::AppState> = app.state();
            state.platform.audio().set_volume(baseline);
        });
    }
}

/// App handle for platform code that has no `AppState` (e.g. media forwarding).
pub fn app_handle() -> Option<&'static AppHandle> {
    APP_HANDLE_REF.get()
}

fn emit_rotate(app: &AppHandle, direction: i32, volume_pct: i32) {
    // Emitter::emit reaches every webview already; emitting again on the window doubles it.
    let _ = app.emit(
        "dial:rotate",
        serde_json::json!({
            "direction": direction,
            "delta": direction,
            "volume": volume_pct,
            "volume_pct": volume_pct,
        }),
    );
}

fn handle_center_click(source: &str) {
    crate::platform::diag::log(&format!("click from {source}"));
    let now = now_millis();
    let action = classify_click(
        now,
        LAST_CLICK_TIME_MS.load(Ordering::SeqCst),
        CLICK_DEBOUNCE_MS,
        flag(&IS_HUD_OPEN_REF),
        flag(&IS_SCROLL_MODE_REF),
    );
    if action == ClickAction::Debounced {
        crate::platform::diag::log(&format!("click from {source} debounced"));
        return;
    }
    crate::platform::diag::log(&format!("click decision: {action:?}"));
    LAST_CLICK_TIME_MS.store(now, Ordering::SeqCst);

    match action {
        ClickAction::ExitScroll => {
            println!(">> [macOS NATIVE DIAL CLICK] -> Exiting Scroll Mode");
            with_app(|app| {
                let state: tauri::State<crate::AppState> = app.state();
                crate::do_stop_scroll_mode(&app, &state);
            });
        }
        ClickAction::PlayPause => {
            println!(">> [macOS NATIVE DIAL CLICK] -> Play/Pause");
            with_app(|app| {
                let state: tauri::State<crate::AppState> = app.state();
                state.platform.media().play_pause();
            });
        }
        ClickAction::Confirm => {
            println!(">> [macOS NATIVE DIAL CLICK] -> Dispatched action select to HUD");
            with_app(|app| {
                let _ = app.emit("dial:click", ());
            });
        }
        ClickAction::Debounced => {}
    }
}

/// Double press (Now Playing Next) and triple press (Previous).
fn handle_multi_press(gesture: MultiPress, source: &str) {
    let now = now_millis();
    let action = classify_multi_press(
        gesture,
        now,
        LAST_GESTURE_TIME_MS.load(Ordering::SeqCst),
        CLICK_DEBOUNCE_MS,
        flag(&IS_HUD_OPEN_REF),
        flag(&IS_SCROLL_MODE_REF),
    );
    crate::platform::diag::log(&format!("{gesture:?} press from {source}: {action:?}"));
    if action == GestureAction::Debounced {
        return;
    }
    LAST_GESTURE_TIME_MS.store(now, Ordering::SeqCst);

    match action {
        GestureAction::OpenHud => {
            // Flip the flag synchronously so rotation ticks arriving while the window
            // animates in are already intercepted.
            if let Some(arc) = IS_HUD_OPEN_REF.get() {
                arc.store(true, Ordering::SeqCst);
            }
            with_app(|app| {
                let state: tauri::State<crate::AppState> = app.state();
                crate::do_show_hud(&app, &state);
            });
        }
        GestureAction::Dismiss => with_app(|app| {
            let state: tauri::State<crate::AppState> = app.state();
            crate::do_hide_hud(&app, &state);
        }),
        GestureAction::ExitScroll => with_app(|app| {
            let state: tauri::State<crate::AppState> = app.state();
            crate::do_stop_scroll_mode(&app, &state);
        }),
        GestureAction::PreviousTrack => with_app(|app| {
            let state: tauri::State<crate::AppState> = app.state();
            state.platform.media().prev_track();
        }),
        GestureAction::Debounced => {}
    }
}

#[cfg(target_os = "macos")]
unsafe fn read_device_volume(device: AudioObjectID) -> Option<i32> {
    let address = AudioObjectPropertyAddress {
        mSelector: kAudioHardwareServiceDeviceProperty_VirtualMasterVolume,
        mScope: kAudioObjectPropertyScopeOutput,
        mElement: kAudioObjectPropertyElementMain,
    };
    let mut vol: f32 = 0.0;
    let mut size = std::mem::size_of::<f32>() as u32;
    let status = AudioObjectGetPropertyData(
        device,
        &address,
        0,
        std::ptr::null(),
        &mut size,
        &mut vol as *mut _ as *mut std::ffi::c_void,
    );
    (status == 0).then(|| ((vol * 100.0).round() as i32).clamp(0, 100))
}

/// CoreAudio listener for the default output device's volume. Bluetooth AVRCP Absolute
/// Volume (the CMF dial) updates the device volume directly instead of posting a media key,
/// so this is the macOS counterpart of the Windows endpoint-volume callback.
#[cfg(target_os = "macos")]
unsafe extern "C" fn volume_listener(
    device: AudioObjectID,
    _count: u32,
    _addresses: *const AudioObjectPropertyAddress,
    _client: *mut std::ffi::c_void,
) -> OSStatus {
    let Some(pct) = read_device_volume(device) else {
        return 0;
    };
    let is_hud_open = flag(&IS_HUD_OPEN_REF);
    let is_scroll = flag(&IS_SCROLL_MODE_REF);
    let baseline = if locked_vol() < 0 { -1 } else { ROTATION_BASELINE.load(Ordering::SeqCst) };

    match classify_volume_change(pct, baseline, is_hud_open, is_scroll) {
        VolumeAction::Turn { direction } => {
            let now = now_millis();
            let burst = is_rotation_burst(now, LAST_ROTATION_MS.load(Ordering::SeqCst));
            if !burst {
                LAST_ROTATION_MS.store(now, Ordering::SeqCst);
            }
            crate::platform::diag::log(&format!(
                "dial turn {pct}% (baseline {baseline}%) dir {direction}{}",
                if burst { " burst" } else { "" }
            ));
            println!(
                ">> [macOS CoreAudio DIAL TURN] {}% (baseline {}%) -> dir {}{}",
                pct,
                baseline,
                direction,
                if burst { " (burst, restore only)" } else { "" }
            );
            // Never set properties from inside a HAL listener; restore on a worker thread.
            with_app(move |app| {
                // The click that ends this navigation must come to us, not to YouTube.
                super::now_playing::reassert_throttled(&app);
                let state: tauri::State<crate::AppState> = app.state();
                state.platform.audio().set_volume(baseline);
                if burst {
                    return;
                }
                if is_scroll {
                    state.platform.input().emit_scroll(scroll_delta_for(direction));
                    let _ = app.emit("scroll:tick", serde_json::json!({ "direction": direction }));
                } else {
                    emit_rotate(&app, direction, locked_vol().max(0));
                }
            });
        }
        VolumeAction::NaturalChange { pct } => {
            with_app(move |app| emit_rotate(&app, 0, pct));
        }
        VolumeAction::Ignore => {}
    }
    0
}

#[cfg(target_os = "macos")]
const VOLUME_ADDRESS: AudioObjectPropertyAddress = AudioObjectPropertyAddress {
    mSelector: kAudioHardwareServiceDeviceProperty_VirtualMasterVolume,
    mScope: kAudioObjectPropertyScopeOutput,
    mElement: kAudioObjectPropertyElementMain,
};

#[cfg(target_os = "macos")]
const DEFAULT_OUTPUT_ADDRESS: AudioObjectPropertyAddress = AudioObjectPropertyAddress {
    mSelector: kAudioHardwarePropertyDefaultOutputDevice,
    mScope: kAudioObjectPropertyScopeGlobal,
    mElement: kAudioObjectPropertyElementMain,
};

/// Moves the volume listener to the current default output (buds connected after launch,
/// switching between speakers and headphones).
#[cfg(target_os = "macos")]
unsafe fn bind_default_output_device() {
    let mut device: AudioObjectID = 0;
    let mut size = std::mem::size_of::<AudioObjectID>() as u32;
    let status = AudioObjectGetPropertyData(
        kAudioObjectSystemObject,
        &DEFAULT_OUTPUT_ADDRESS,
        0,
        std::ptr::null(),
        &mut size,
        &mut device as *mut _ as *mut std::ffi::c_void,
    );
    if status != 0 || device == 0 {
        return;
    }
    let previous = BOUND_OUTPUT_DEVICE.swap(device, Ordering::SeqCst);
    if previous == device {
        return;
    }
    if previous != 0 {
        AudioObjectRemovePropertyListener(previous, &VOLUME_ADDRESS, volume_listener, std::ptr::null_mut());
    }
    let status = AudioObjectAddPropertyListener(device, &VOLUME_ADDRESS, volume_listener, std::ptr::null_mut());
    if status == 0 {
        println!("[+] macOS CoreAudio volume listener bound to output device {}", device);
    } else {
        eprintln!("[!] macOS: could not listen to volume of device {} (OSStatus {})", device, status);
    }
}

#[cfg(target_os = "macos")]
unsafe extern "C" fn default_output_listener(
    _object: AudioObjectID,
    _count: u32,
    _addresses: *const AudioObjectPropertyAddress,
    _client: *mut std::ffi::c_void,
) -> OSStatus {
    bind_default_output_device();
    0
}

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

        if event_type == kCGEventTapDisabledByUserInput || event_type != NX_SYSDEFINED {
            return event;
        }

        let mut subtype: i16 = 0;
        let mut data1: isize = 0;

        // This callback runs on a plain thread with no autorelease pool; without one every
        // NSEvent created below would leak.
        let pool = objc_autoreleasePoolPush();
        let cls_nsevent = objc_getClass(b"NSEvent\0".as_ptr() as *const _);
        if !cls_nsevent.is_null() {
            let sel_event_with_cg = sel_registerName(b"eventWithCGEvent:\0".as_ptr() as *const _);
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
        objc_autoreleasePoolPop(pool);

        if subtype != NX_SUBTYPE_AUX_CONTROL_BUTTONS {
            return event;
        }

        // Media keys we post ourselves (HUD media slices) must reach the player, not the dial.
        if CGEventGetIntegerValueField(event, kCGEventSourceUnixProcessID) == std::process::id() as i64 {
            return event;
        }

        let key = decode_aux_key(data1 as i64);

        // 1. Center Click / Media Play-Pause Key: dropped here; handle_center_click forwards it to
        //    the player itself when the HUD is closed.
        if key.key_type == NX_KEYTYPE_PLAY {
            if key.is_down && !key.is_repeat {
                handle_center_click("event tap");
            }
            return std::ptr::null_mut();
        }

        // 2. Rotary Dial Rotation (Volume Up = CW, Volume Down = CCW)
        if key.key_type == NX_KEYTYPE_SOUND_UP || key.key_type == NX_KEYTYPE_SOUND_DOWN {
            let dir = if key.key_type == NX_KEYTYPE_SOUND_UP { 1 } else { -1 };
            let is_scroll = flag(&IS_SCROLL_MODE_REF);
            let is_hud_open = flag(&IS_HUD_OPEN_REF);

            if is_scroll {
                if key.is_down {
                    with_app(move |app| {
                        let state: tauri::State<crate::AppState> = app.state();
                        state.platform.input().emit_scroll(scroll_delta_for(dir));
                        let _ = app.emit("scroll:tick", serde_json::json!({ "direction": dir }));
                    });
                }
                // Drop event (both down and up): prevent OS volume change
                return std::ptr::null_mut();
            }

            if is_hud_open {
                if key.is_down {
                    let locked_val = locked_vol();
                    with_app(move |app| {
                        let state: tauri::State<crate::AppState> = app.state();
                        let shown = if locked_val < 0 {
                            // Volume mode: apply the change ourselves since the key is dropped.
                            let current = state.platform.audio().get_volume();
                            state.platform.audio().set_volume((current + dir * 2).clamp(0, 100))
                        } else {
                            locked_val
                        };
                        emit_rotate(&app, dir, shown);
                    });
                }
                // Always drop while the HUD is open so macOS doesn't change volume or show its HUD
                return std::ptr::null_mut();
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

        // Earbud Play/Pause/Next/Previous go to the Now Playing app, never to the event tap,
        // so become that app. MediaPlayer handlers are registered and invoked on the main thread.
        #[cfg(target_os = "macos")]
        if let Some(app) = APP_HANDLE_REF.get() {
            let _ = app.run_on_main_thread(|| {
                super::now_playing::start(|cmd| match cmd {
                    RemoteCommand::Next => handle_multi_press(MultiPress::Double, "now playing"),
                    RemoteCommand::Previous => handle_multi_press(MultiPress::Triple, "now playing"),
                    cmd if crate::platform::dial_logic::remote_command_is_click(cmd) => {
                        handle_center_click("now playing")
                    }
                    _ => {}
                });
            });
        }

        #[cfg(target_os = "macos")]
        unsafe {
            // HAL delivers property notifications on its own thread; no run loop needed.
            bind_default_output_device();
            AudioObjectAddPropertyListener(
                kAudioObjectSystemObject,
                &DEFAULT_OUTPUT_ADDRESS,
                default_output_listener,
                std::ptr::null_mut(),
            );
        }

        #[cfg(target_os = "macos")]
        thread::spawn(move || unsafe {
            let event_mask: CGEventMask = 1 << NX_SYSDEFINED;
            let mut port: CFMachPortRef = std::ptr::null_mut();

            // Retry every 2 seconds while Accessibility permission is pending
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

            TAP_PORT_REF.store(port as u64, Ordering::SeqCst);

            println!("[+] macOS CGEventTap for rotary dial & media keys initialized successfully.");

            let run_loop_source = CFMachPortCreateRunLoopSource(std::ptr::null(), port, 0);
            let run_loop = CFRunLoopGetCurrent();

            CFRunLoopAddSource(run_loop, run_loop_source, kCFRunLoopCommonModes);
            CGEventTapEnable(port, true);

            CFRunLoopRun();
        });

        #[cfg(not(target_os = "macos"))]
        {
            let _ = Duration::from_secs(0);
            println!("[macOS Mock Dial] Listeners registered");
        }
    }
}
