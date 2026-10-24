//! Own "Now Playing" session (MediaPlayer framework).
//!
//! Bluetooth AVRCP play/pause/next/previous from the earbuds never become `NX_SYSDEFINED`
//! key events on macOS, so the event tap cannot see them: `mediaremoted` routes them to the
//! current Now Playing app (Music, Spotify, a browser tab), which toggles playback. We become
//! the Now Playing app ourselves (`MPRemoteCommandCenter` handlers + an explicit
//! `MPNowPlayingInfoCenter.playbackState`, which macOS requires instead of inferring it), so
//! the presses reach the dial logic. This is the macOS counterpart of `windows/smtc.rs`.

use crate::platform::dial_logic::{now_millis, reported_playing_after, RemoteCommand};
use block2::RcBlock;
use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::OnceLock;

use super::ffi::{objc_getClass, objc_msgSend, sel_registerName};

#[link(name = "MediaPlayer", kind = "framework")]
extern "C" {
    static MPMediaItemPropertyTitle: *mut c_void;
}

const MP_HANDLER_SUCCESS: isize = 0;
const MP_PLAYBACK_PLAYING: usize = 1;
const MP_PLAYBACK_PAUSED: usize = 2;
const MP_PLAYBACK_STOPPED: usize = 4;

/// State the earbuds expect us to be in (see `reported_playing_after`).
static REPORT_PLAYING: AtomicBool = AtomicBool::new(true);
static ACTIVE: AtomicBool = AtomicBool::new(false);
/// While we have stepped aside so a media slice reaches the real player, presses that still
/// arrive here are that forwarded command, not the dial.
static FORWARDING_UNTIL_MS: AtomicU64 = AtomicU64::new(0);
static ON_COMMAND: OnceLock<Box<dyn Fn(RemoteCommand) + Send + Sync>> = OnceLock::new();

fn sel(name: &[u8]) -> *mut c_void {
    unsafe { sel_registerName(name.as_ptr() as *const _) }
}

fn class(name: &[u8]) -> *mut c_void {
    unsafe { objc_getClass(name.as_ptr() as *const _) }
}

unsafe fn send0(obj: *mut c_void, s: &[u8]) -> *mut c_void {
    let f: unsafe extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void =
        std::mem::transmute(objc_msgSend as *const ());
    f(obj, sel(s))
}

unsafe fn send_ptr(obj: *mut c_void, s: &[u8], arg: *mut c_void) -> *mut c_void {
    let f: unsafe extern "C" fn(*mut c_void, *mut c_void, *mut c_void) -> *mut c_void =
        std::mem::transmute(objc_msgSend as *const ());
    f(obj, sel(s), arg)
}

unsafe fn send_bool(obj: *mut c_void, s: &[u8], arg: bool) {
    let f: unsafe extern "C" fn(*mut c_void, *mut c_void, i8) =
        std::mem::transmute(objc_msgSend as *const ());
    f(obj, sel(s), arg as i8)
}

unsafe fn send_usize(obj: *mut c_void, s: &[u8], arg: usize) {
    let f: unsafe extern "C" fn(*mut c_void, *mut c_void, usize) =
        std::mem::transmute(objc_msgSend as *const ());
    f(obj, sel(s), arg)
}

unsafe fn nsstring(text: &str) -> *mut c_void {
    let c = std::ffi::CString::new(text).unwrap_or_default();
    send_ptr(class(b"NSString\0"), b"stringWithUTF8String:\0", c.as_ptr() as *mut c_void)
}

/// Publishes our session to `MPNowPlayingInfoCenter`. Must run on the main thread.
unsafe fn publish(state: usize) {
    let center = send0(class(b"MPNowPlayingInfoCenter\0"), b"defaultCenter\0");
    if center.is_null() {
        return;
    }
    if state == MP_PLAYBACK_STOPPED {
        send_ptr(center, b"setNowPlayingInfo:\0", std::ptr::null_mut());
    } else {
        let f: unsafe extern "C" fn(*mut c_void, *mut c_void, *mut c_void, *mut c_void) -> *mut c_void =
            std::mem::transmute(objc_msgSend as *const ());
        let info = f(
            class(b"NSDictionary\0"),
            sel(b"dictionaryWithObject:forKey:\0"),
            nsstring("Dial Control"),
            MPMediaItemPropertyTitle,
        );
        send_ptr(center, b"setNowPlayingInfo:\0", info);
    }
    send_usize(center, b"setPlaybackState:\0", state);
}

fn reported_state() -> usize {
    if REPORT_PLAYING.load(Ordering::SeqCst) {
        MP_PLAYBACK_PLAYING
    } else {
        MP_PLAYBACK_PAUSED
    }
}

fn handle(cmd: RemoteCommand) -> isize {
    if now_millis() < FORWARDING_UNTIL_MS.load(Ordering::SeqCst) {
        crate::platform::diag::log(&format!("now playing: {cmd:?} ignored while forwarding"));
        return MP_HANDLER_SUCCESS;
    }
    crate::platform::diag::log(&format!("now playing: remote command {cmd:?}"));
    let playing = reported_playing_after(cmd, REPORT_PLAYING.load(Ordering::SeqCst));
    REPORT_PLAYING.store(playing, Ordering::SeqCst);
    // Handlers run on the main thread, so the state can be published right away.
    unsafe { publish(reported_state()) };
    if let Some(cb) = ON_COMMAND.get() {
        cb(cmd);
    }
    MP_HANDLER_SUCCESS
}

unsafe fn register(center: *mut c_void, getter: &[u8], cmd: RemoteCommand) {
    let command = send0(center, getter);
    if command.is_null() {
        return;
    }
    send_bool(command, b"setEnabled:\0", true);
    let block = RcBlock::new(move |_event: *mut c_void| -> isize { handle(cmd) });
    // The command center copies (retains) the handler block.
    send_ptr(command, b"addTargetWithHandler:\0", RcBlock::as_ptr(&block) as *mut c_void);
}

/// Registers the remote command handlers and claims Now Playing. Call on the main thread.
pub fn start<F: Fn(RemoteCommand) + Send + Sync + 'static>(on_command: F) {
    if ON_COMMAND.set(Box::new(on_command)).is_err() {
        return;
    }
    unsafe {
        let center = send0(class(b"MPRemoteCommandCenter\0"), b"sharedCommandCenter\0");
        if center.is_null() {
            crate::platform::diag::log("now playing: MPRemoteCommandCenter unavailable");
            return;
        }
        register(center, b"togglePlayPauseCommand\0", RemoteCommand::Toggle);
        register(center, b"playCommand\0", RemoteCommand::Play);
        register(center, b"pauseCommand\0", RemoteCommand::Pause);
        register(center, b"nextTrackCommand\0", RemoteCommand::Next);
        register(center, b"previousTrackCommand\0", RemoteCommand::Previous);
        publish(reported_state());
    }
    ACTIVE.store(true, Ordering::SeqCst);
    crate::platform::diag::log("now playing: session registered");
}

/// Re-publishes our state so we become the most recent Now Playing app again (after the user
/// played something elsewhere). Cheap; call when the dial is about to need clicks.
pub fn reassert(app: &tauri::AppHandle) {
    if ACTIVE.load(Ordering::SeqCst) {
        let _ = app.run_on_main_thread(|| unsafe { publish(reported_state()) });
    }
}

/// Steps aside so a media key from a HUD slice reaches the real player instead of us (media
/// keys go to the Now Playing app). `send` posts the key; we reclaim the session afterwards.
pub fn forward_to_player(app: &tauri::AppHandle, send: impl FnOnce()) {
    if !ACTIVE.load(Ordering::SeqCst) {
        send();
        return;
    }
    FORWARDING_UNTIL_MS.store(now_millis() + 1500, Ordering::SeqCst);
    let _ = app.run_on_main_thread(|| unsafe { publish(MP_PLAYBACK_STOPPED) });
    std::thread::sleep(std::time::Duration::from_millis(200));
    send();
    crate::platform::diag::log("now playing: forwarded media key to the real player");
    std::thread::sleep(std::time::Duration::from_millis(1300));
    reassert(app);
}
