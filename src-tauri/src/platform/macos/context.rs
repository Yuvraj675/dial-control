use crate::platform::common::{match_profile, ContextEvent};
use crate::platform::traits::ContextDetector;
use std::process::Command;
use std::sync::atomic::{AtomicI32, Ordering};
use std::thread;
use std::time::Duration;
use tauri::{AppHandle, Emitter};

#[cfg(target_os = "macos")]
use super::ffi::*;

const OWN_BUNDLE_ID: &str = "com.yuvraj.dialcontrol";
const OWN_APP_NAME: &str = "Dial Control";

/// PID of the last foreground app that was not Dial Control (hotkey focus target).
static LAST_FOREIGN_PID: AtomicI32 = AtomicI32::new(0);

pub struct MacContext;

struct FrontWindow {
    app_name: String,
    bundle_id: String,
    title: String,
    pid: i32,
}

impl MacContext {
    pub fn new() -> Self {
        Self
    }

    fn query_frontmost_window() -> Option<FrontWindow> {
        // Queries active application name, bundle ID, window title and pid via osascript
        let script = r#"
            try
                tell application "System Events"
                    set frontApp to first application process whose frontmost is true
                    set appName to name of frontApp
                    set bundleID to bundle identifier of frontApp
                    set appPid to unix id of frontApp
                    set winTitle to ""
                    try
                        set winTitle to name of front window of frontApp
                    end try
                    return appName & "|||" & bundleID & "|||" & winTitle & "|||" & appPid
                end tell
            on error
                return ""
            end try
        "#;

        let output = Command::new("osascript").args(["-e", script]).output().ok()?;
        parse_frontmost_output(&String::from_utf8_lossy(&output.stdout))
    }
}

fn parse_frontmost_output(text: &str) -> Option<FrontWindow> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return None;
    }
    let parts: Vec<&str> = trimmed.split("|||").map(str::trim).collect();
    let part = |i: usize| parts.get(i).map(|s| s.to_string()).unwrap_or_default();
    Some(FrontWindow {
        app_name: part(0),
        bundle_id: part(1),
        title: part(2),
        pid: parts.get(3).and_then(|p| p.parse().ok()).unwrap_or(0),
    })
}

fn is_own_app(front: &FrontWindow) -> bool {
    front.bundle_id == OWN_BUNDLE_ID
        || front.app_name == OWN_APP_NAME
        || front.app_name == "dial-control"
        || (front.pid > 0 && front.pid as u32 == std::process::id())
}

/// Re-activates the app the user was in before the HUD took focus. Must run on the main
/// thread (called via `run_on_main_thread`).
/// Lets the HUD appear over full-screen apps: status window level, joins every Space
/// (including full-screen ones) and never hides when the app deactivates. Main thread.
#[cfg(target_os = "macos")]
pub fn raise_over_fullscreen(ns_window: *mut std::ffi::c_void) {
    const NS_STATUS_WINDOW_LEVEL: isize = 25;
    const CAN_JOIN_ALL_SPACES: usize = 1 << 0;
    const STATIONARY: usize = 1 << 4;
    const IGNORES_CYCLE: usize = 1 << 6;
    const FULL_SCREEN_AUXILIARY: usize = 1 << 8;
    if ns_window.is_null() {
        return;
    }
    unsafe {
        type SendIsize = unsafe extern "C" fn(*mut std::ffi::c_void, *mut std::ffi::c_void, isize);
        type SendUsize = unsafe extern "C" fn(*mut std::ffi::c_void, *mut std::ffi::c_void, usize);
        type SendBool = unsafe extern "C" fn(*mut std::ffi::c_void, *mut std::ffi::c_void, i8);
        type Send0 = unsafe extern "C" fn(*mut std::ffi::c_void, *mut std::ffi::c_void);
        let s_isize: SendIsize = std::mem::transmute(objc_msgSend as *const ());
        let s_usize: SendUsize = std::mem::transmute(objc_msgSend as *const ());
        let s_bool: SendBool = std::mem::transmute(objc_msgSend as *const ());
        let s0: Send0 = std::mem::transmute(objc_msgSend as *const ());
        let sel = |n: &[u8]| sel_registerName(n.as_ptr() as *const _);
        s_isize(ns_window, sel(b"setLevel:\0"), NS_STATUS_WINDOW_LEVEL);
        s_usize(
            ns_window,
            sel(b"setCollectionBehavior:\0"),
            CAN_JOIN_ALL_SPACES | STATIONARY | IGNORES_CYCLE | FULL_SCREEN_AUXILIARY,
        );
        s_bool(ns_window, sel(b"setHidesOnDeactivate:\0"), 0);
        s0(ns_window, sel(b"orderFrontRegardless\0"));
    }
}

pub fn reactivate_previous_app() {
    let pid = LAST_FOREIGN_PID.load(Ordering::SeqCst);
    if pid <= 0 {
        return;
    }
    #[cfg(target_os = "macos")]
    unsafe {
        let cls = objc_getClass(b"NSRunningApplication\0".as_ptr() as *const _);
        if cls.is_null() {
            return;
        }
        let sel_by_pid =
            sel_registerName(b"runningApplicationWithProcessIdentifier:\0".as_ptr() as *const _);
        let sel_activate = sel_registerName(b"activateWithOptions:\0".as_ptr() as *const _);

        type MsgSendPid =
            unsafe extern "C" fn(*mut std::ffi::c_void, *mut std::ffi::c_void, i32) -> *mut std::ffi::c_void;
        type MsgSendOptions =
            unsafe extern "C" fn(*mut std::ffi::c_void, *mut std::ffi::c_void, usize) -> i8;
        let by_pid: MsgSendPid = std::mem::transmute(objc_msgSend as *const ());
        let activate: MsgSendOptions = std::mem::transmute(objc_msgSend as *const ());

        let running_app = by_pid(cls, sel_by_pid, pid);
        if !running_app.is_null() {
            // NSApplicationActivateIgnoringOtherApps
            activate(running_app, sel_activate, 1 << 1);
        }
    }
}

impl ContextDetector for MacContext {
    fn start(&self, app: AppHandle) {
        thread::spawn(move || {
            let mut last_app = String::new();
            let mut last_title = String::new();

            loop {
                thread::sleep(Duration::from_millis(500));

                let Some(front) = Self::query_frontmost_window() else {
                    continue;
                };

                // Our own HUD/Settings becoming frontmost must not reset the profile, and must
                // never become the hotkey focus target.
                if is_own_app(&front) {
                    continue;
                }
                if front.pid > 0 {
                    LAST_FOREIGN_PID.store(front.pid, Ordering::SeqCst);
                }

                if front.app_name != last_app {
                    // A newly focused app (a browser with YouTube) may have taken Now Playing.
                    super::now_playing::reassert(&app);
                }

                if front.app_name != last_app || front.title != last_title {
                    let profile_id = match_profile(&front.app_name, &front.bundle_id, &front.title);

                    println!(
                        "[macOS Context] Active -> App: '{}', Bundle: '{}', Title: '{}' -> Matched Profile: '{}'",
                        front.app_name, front.bundle_id, front.title, profile_id
                    );

                    last_app = front.app_name.clone();
                    last_title = front.title.clone();

                    let _ = app.emit(
                        "context:change",
                        ContextEvent {
                            profile_id: profile_id.to_string(),
                            app: front.app_name,
                            title: front.title,
                            wm_class: front.bundle_id,
                        },
                    );
                }
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_full_osascript_line() {
        let f = parse_frontmost_output("Safari|||com.apple.Safari|||Apple|||412\n").unwrap();
        assert_eq!(f.app_name, "Safari");
        assert_eq!(f.bundle_id, "com.apple.Safari");
        assert_eq!(f.title, "Apple");
        assert_eq!(f.pid, 412);
    }

    #[test]
    fn parses_missing_fields() {
        let f = parse_frontmost_output("Finder|||com.apple.finder").unwrap();
        assert_eq!(f.title, "");
        assert_eq!(f.pid, 0);
        assert!(parse_frontmost_output("   \n").is_none());
    }

    #[test]
    fn own_app_is_filtered() {
        let own = parse_frontmost_output("Dial Control|||com.yuvraj.dialcontrol|||Dial Control HUD|||1").unwrap();
        assert!(is_own_app(&own));
        let other = parse_frontmost_output("Code|||com.microsoft.VSCode|||main.rs|||2").unwrap();
        assert!(!is_own_app(&other));
    }
}
