use crate::platform::dial_logic::{
    classify_click, classify_floating_change, consumer_click_edge, now_millis, ClickAction,
    FloatingAction, CLICK_DEBOUNCE_MS, FLOAT_RECENTER_LEVEL,
};
use crate::platform::traits::{DialBackend, PlatformProvider};
use crate::platform::windows::ffi::*;
use crate::AppState;
use std::cell::RefCell;
use std::collections::HashMap;
use std::ffi::c_void;
use std::ptr::null_mut;
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU32, AtomicUsize, Ordering};
use std::sync::{Arc, OnceLock, RwLock};
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
/// True between one of our own volume sets and the next real turn (see classify_floating_change).
static ECHO_ARMED: AtomicBool = AtomicBool::new(false);

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
    ECHO_ARMED.store(true, Ordering::SeqCst);
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

fn read_flag(slot: &RwLock<Option<Arc<AtomicBool>>>) -> bool {
    slot.read()
        .ok()
        .and_then(|g| g.as_ref().map(|a| a.load(Ordering::SeqCst)))
        .unwrap_or(false)
}

fn read_locked_vol() -> i32 {
    LOCKED_VOL_REF
        .read()
        .ok()
        .and_then(|g| g.as_ref().map(|a| a.load(Ordering::SeqCst)))
        .unwrap_or(-1)
}

/// Runs `f` with the app handle on a worker thread so hooks and COM callbacks return fast.
fn with_app<F: FnOnce(AppHandle) + Send + 'static>(f: F) {
    if let Ok(guard) = APP_HANDLE_REF.read() {
        if let Some(ref app) = *guard {
            let app = app.clone();
            thread::spawn(move || f(app));
        }
    }
}

/// Single entry point for a dial center click, fed by both the low-level keyboard hook
/// (VK_MEDIA_PLAY_PAUSE) and Raw Input (HID Consumer Play / Pause). One physical press can
/// be observed by both, so the debounce timestamp is claimed with a compare-exchange and
/// only the first observer acts.
fn handle_dial_click(source: &str) {
    let now = now_millis();
    let last = LAST_DIAL_CLICK_TIME.load(Ordering::SeqCst);
    let action = classify_click(
        now,
        last as u64,
        CLICK_DEBOUNCE_MS,
        read_flag(&IS_HUD_OPEN_REF),
        read_flag(&IS_SCROLL_MODE_REF),
    );
    if action == ClickAction::Debounced {
        return;
    }
    if LAST_DIAL_CLICK_TIME
        .compare_exchange(last, now as usize, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return;
    }

    match action {
        ClickAction::ExitScroll => {
            println!("[Windows Dial] Dial click ({}) -> Exiting Scroll Mode", source);
            with_app(|app| {
                let state: tauri::State<AppState> = app.state();
                crate::do_stop_scroll_mode(&app, &state);
            });
        }
        ClickAction::OpenHud => {
            // Flip the flag immediately so rotation arriving before the window is up is intercepted.
            if let Ok(guard) = IS_HUD_OPEN_REF.read() {
                if let Some(ref is_open) = *guard {
                    is_open.store(true, Ordering::SeqCst);
                }
            }
            HAS_ROTATED.store(false, Ordering::SeqCst);
            unsafe {
                AllowSetForegroundWindow(GetCurrentProcessId());
            }
            println!("[Windows Dial] Dial click ({}) -> Opening HUD", source);
            with_app(|app| {
                let state: tauri::State<AppState> = app.state();
                crate::do_show_hud(&app, &state);
            });
        }
        ClickAction::Confirm if !HAS_ROTATED.load(Ordering::SeqCst) => {
            // Clicked again without rotating -> toggle the HUD closed.
            println!("[Windows Dial] Dial click ({}) without rotation -> Toggling HUD closed", source);
            with_app(|app| {
                let state: tauri::State<AppState> = app.state();
                crate::do_hide_hud(&app, &state);
            });
        }
        ClickAction::Confirm => {
            println!("[Windows Dial] Dial click ({}) -> Confirm selection", source);
            with_app(|app| {
                let _ = app.emit("dial:click", ());
            });
        }
        ClickAction::Debounced => {}
    }
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
                // Consume both KeyDown and KeyUp so media players (Spotify, YouTube, Chrome)
                // never receive a play/pause toggle when clicking the physical dial.
                if is_key_down {
                    handle_dial_click("keyboard hook");
                }
                return 1;
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
            // In volume mode (HUD open, lock released) the keys pass through so Windows changes
            // the volume natively and the endpoint callback mirrors it on the gauge.
            VK_VOLUME_UP | VK_VOLUME_DOWN if is_scroll_mode || (is_hud_open && read_locked_vol() >= 0) => {
                if is_key_up {
                    return 1;
                }
                if is_key_down {
                    let dir = if vk == VK_VOLUME_UP { 1 } else { -1 };
                    HAS_ROTATED.store(true, Ordering::SeqCst);

                    if is_scroll_mode {
                        let scroll_delta = crate::platform::dial_logic::scroll_delta_for(dir);
                        println!("[Windows Dial] Keyboard Scroll Mode -> dir: {}, delta: {}", dir, scroll_delta);
                        with_app(move |app| {
                            let state: tauri::State<AppState> = app.state();
                            state.platform.input().emit_scroll(scroll_delta);
                            let _ = app.emit("scroll:tick", serde_json::json!({ "direction": dir }));
                        });
                        return 1;
                    }

                    println!("[Windows Dial] Intercepted Keyboard Volume -> ({})", dir);
                    let locked_val = read_locked_vol();
                    with_app(move |app| {
                        let _ = app.emit(
                            "dial:rotate",
                            RotatePayload {
                                direction: dir,
                                delta: dir,
                                volume: locked_val,
                                volume_pct: locked_val,
                            },
                        );
                    });
                    return 1;
                }
            }
            _ => {}
        }
    }

    CallNextHookEx(hhook, n_code, w_param, l_param)
}

// ── Raw Input Consumer-Control Click Listener ────────────────────────────────
// AVRCP has no play/pause toggle opcode, so the dial click arrives as discrete PLAY or
// PAUSE (Linux sees KEY_PLAYCD / KEY_PAUSECD). Windows maps HID Consumer Play (0xB0) and
// Pause (0xB1) to WM_APPCOMMAND only: no virtual key is generated, so WH_KEYBOARD_LL never
// sees the click. Consumer Control is a *shared* HID collection, so a RIDEV_INPUTSINK
// registration observes those reports from every device without stealing them.
// Limitation: Raw Input can observe but not swallow, so a discrete Play/Pause may still
// reach the active media session; VK_MEDIA_PLAY_PAUSE clicks remain fully swallowed by the hook.

static HIDP_GET_USAGES: OnceLock<Option<FnHidPGetUsages>> = OnceLock::new();

thread_local! {
    static PREPARSED_DATA: RefCell<HashMap<usize, Vec<u64>>> = RefCell::new(HashMap::new());
    static CLICK_DOWN: RefCell<HashMap<usize, bool>> = RefCell::new(HashMap::new());
}

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

fn hidp_get_usages() -> Option<FnHidPGetUsages> {
    *HIDP_GET_USAGES.get_or_init(|| unsafe {
        let module = LoadLibraryW(wide("hid.dll").as_ptr());
        if module.is_null() {
            return None;
        }
        let proc_addr = GetProcAddress(module, b"HidP_GetUsages\0".as_ptr());
        if proc_addr.is_null() {
            return None;
        }
        Some(std::mem::transmute::<*const c_void, FnHidPGetUsages>(proc_addr))
    })
}

/// Returns a pointer to this device's cached HIDP_PREPARSED_DATA (fetched once per device).
unsafe fn preparsed_data_for(device: HANDLE) -> Option<*mut c_void> {
    PREPARSED_DATA.with(|cache| {
        let mut cache = cache.borrow_mut();
        if let Some(buf) = cache.get_mut(&(device as usize)) {
            return Some(buf.as_mut_ptr() as *mut c_void);
        }
        let mut size: u32 = 0;
        if GetRawInputDeviceInfoW(device, RIDI_PREPARSEDDATA, null_mut(), &mut size) != 0 || size == 0 {
            return None;
        }
        // u64 storage keeps the opaque blob 8-byte aligned.
        let mut buf: Vec<u64> = vec![0; (size as usize + 7) / 8];
        if GetRawInputDeviceInfoW(device, RIDI_PREPARSEDDATA, buf.as_mut_ptr() as *mut c_void, &mut size) == u32::MAX {
            return None;
        }
        let ptr = buf.as_mut_ptr() as *mut c_void;
        cache.insert(device as usize, buf);
        Some(ptr)
    })
}

unsafe fn process_raw_input(h_raw_input: HANDLE) {
    let header_size = std::mem::size_of::<RAWINPUTHEADER>() as u32;
    let mut size: u32 = 0;
    if GetRawInputData(h_raw_input, RID_INPUT, null_mut(), &mut size, header_size) != 0 || size == 0 {
        return;
    }
    let mut buf: Vec<u64> = vec![0; (size as usize + 7) / 8];
    let copied = GetRawInputData(h_raw_input, RID_INPUT, buf.as_mut_ptr() as *mut c_void, &mut size, header_size);
    if copied == u32::MAX || copied < header_size + 8 {
        return;
    }

    let header = *(buf.as_ptr() as *const RAWINPUTHEADER);
    if header.dwType != RIM_TYPEHID {
        return;
    }
    let base = buf.as_mut_ptr() as *mut u8;
    let hid = *(base.add(header_size as usize) as *const RAWHID_HEADER);
    let reports = base.add(header_size as usize + std::mem::size_of::<RAWHID_HEADER>());
    let total = (hid.dwSizeHid as usize).saturating_mul(hid.dwCount as usize);
    if hid.dwSizeHid == 0 || header_size as usize + 8 + total > copied as usize {
        return;
    }

    let (Some(get_usages), Some(preparsed)) = (hidp_get_usages(), preparsed_data_for(header.hDevice)) else {
        return;
    };

    for i in 0..hid.dwCount as usize {
        let report = reports.add(i * hid.dwSizeHid as usize);
        let mut usages = [0u16; 32];
        let mut count = usages.len() as u32;
        let status = get_usages(
            HIDP_INPUT,
            HID_USAGE_PAGE_CONSUMER,
            0,
            usages.as_mut_ptr(),
            &mut count,
            preparsed,
            report,
            hid.dwSizeHid,
        );
        // Reports with another report ID return HIDP_STATUS_INCOMPATIBLE_REPORT_ID: not ours.
        if status != HIDP_STATUS_SUCCESS {
            continue;
        }
        let pressed = &usages[..count.min(32) as usize];
        let device = header.hDevice as usize;
        let edge = CLICK_DOWN.with(|state| {
            let mut state = state.borrow_mut();
            let was_down = state.get(&device).copied().unwrap_or(false);
            let (edge, now_down) = consumer_click_edge(was_down, pressed);
            state.insert(device, now_down);
            edge
        });
        if edge {
            println!("[Windows Dial] Raw Input consumer click usages: {:02X?}", pressed);
            handle_dial_click("raw input");
        }
    }
}

unsafe extern "system" fn raw_input_wnd_proc(hwnd: HWND, msg: u32, w_param: WPARAM, l_param: LPARAM) -> LRESULT {
    if msg == WM_INPUT {
        process_raw_input(l_param as HANDLE);
    }
    DefWindowProcW(hwnd, msg, w_param, l_param)
}

fn run_raw_input_click_listener() {
    thread::spawn(move || unsafe {
        let h_instance = GetModuleHandleW(null_mut());
        let class_name = wide("DialControlRawInputSink");
        let mut wc: WNDCLASSW = std::mem::zeroed();
        wc.lpfnWndProc = Some(raw_input_wnd_proc);
        wc.hInstance = h_instance;
        wc.lpszClassName = class_name.as_ptr();
        if RegisterClassW(&wc) == 0 {
            eprintln!("[Windows Dial] Raw Input: RegisterClassW failed; consumer clicks unavailable.");
            return;
        }

        let hwnd = CreateWindowExW(
            0,
            class_name.as_ptr(),
            class_name.as_ptr(),
            0,
            0,
            0,
            0,
            0,
            HWND_MESSAGE,
            null_mut(),
            h_instance,
            null_mut(),
        );
        if hwnd.is_null() {
            eprintln!("[Windows Dial] Raw Input: could not create message-only window.");
            return;
        }

        let device = RAWINPUTDEVICE {
            usUsagePage: HID_USAGE_PAGE_CONSUMER,
            usUsage: HID_USAGE_CONSUMER_CONTROL,
            dwFlags: RIDEV_INPUTSINK,
            hwndTarget: hwnd,
        };
        if RegisterRawInputDevices(&device, 1, std::mem::size_of::<RAWINPUTDEVICE>() as u32) == 0 {
            eprintln!("[Windows Dial] Raw Input: RegisterRawInputDevices failed for Consumer Control.");
            return;
        }
        println!("[Windows Dial] Raw Input consumer-control listener active (AVRCP Play/Pause clicks).");

        let mut msg: MSG = std::mem::zeroed();
        while GetMessageW(&mut msg, null_mut(), 0, 0) > 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    });
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

    let now = now_millis() as usize;
    let is_hud_open = read_flag(&IS_HUD_OPEN_REF);
    let is_scroll_mode = read_flag(&IS_SCROLL_MODE_REF);
    let locked_val = read_locked_vol();
    // Volume mode = HUD open with the lock released: the change is the user's real intent.
    let is_volume_mode = is_hud_open && !is_scroll_mode && locked_val < 0;

    // 2. Decide via the shared, unit-tested floating-headroom model (dial_logic). It also
    // swallows Windows' GUID_NULL duplicate of our own set (~1ms later).
    let prev = get_last_audio_level();
    let action = classify_floating_change(
        new_vol,
        prev,
        get_last_set_level(),
        now.saturating_sub(LAST_SET_TIME.load(Ordering::SeqCst)) as u64,
        ECHO_ARMED.load(Ordering::SeqCst),
        is_hud_open,
        is_scroll_mode,
        is_volume_mode,
    );
    let (dir, recenter) = match action {
        FloatingAction::Track => {
            set_last_audio_level(new_vol);
            return 0;
        }
        FloatingAction::Jitter => return 0,
        FloatingAction::Turn { direction, recenter } => (direction, recenter),
    };

    set_last_audio_level(new_vol);
    // A real turn consumes the echo window: turning straight back onto the level we last
    // set must count as a detent, not as Windows' duplicate notification.
    ECHO_ARMED.store(false, Ordering::SeqCst);
    HAS_ROTATED.store(true, Ordering::SeqCst);
    LAST_AUDIO_NOTIFY_TIME.store(now, Ordering::SeqCst);

    println!(
        "[Windows Dial] Rotary turn detected: {:.1}% (prev: {:.1}%, dir: {})",
        new_vol * 100.0,
        prev * 100.0,
        dir
    );

    // Headroom safety: near 0% or 100%, recenter to 50% (never in volume mode).
    if recenter {
        println!("[Windows Dial] Volume near boundary ({:.1}%) -> Recentering to 50% for headroom", new_vol * 100.0);
        let _ = crate::platform::windows::audio::set_hardware_volume_with_context(
            50,
            &DIAL_CONTROL_EVENT_CONTEXT,
        );
        set_last_set_level(FLOAT_RECENTER_LEVEL);
        LAST_SET_TIME.store(now, Ordering::SeqCst);
        ECHO_ARMED.store(true, Ordering::SeqCst);
        set_last_audio_level(FLOAT_RECENTER_LEVEL);
    }

    with_app(move |app| {
        let state: tauri::State<AppState> = app.state();

        if is_scroll_mode {
            let scroll_delta = crate::platform::dial_logic::scroll_delta_for(dir);
            state.platform.input().emit_scroll(scroll_delta);
            let _ = app.emit("scroll:tick", serde_json::json!({ "direction": dir }));
        } else if is_hud_open {
            let shown = if locked_val >= 0 { locked_val } else { (new_vol * 100.0).round() as i32 };
            // Emitter::emit already reaches every webview; a second window.emit would
            // deliver each detent twice and skip a slice per turn.
            let _ = app.emit(
                "dial:rotate",
                RotatePayload {
                    direction: dir,
                    delta: dir,
                    volume: shown,
                    volume_pct: shown,
                },
            );
        }
    });

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

        // 3. Observe HID Consumer Play/Pause clicks that never reach the keyboard hook
        run_raw_input_click_listener();

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
