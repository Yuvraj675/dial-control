//! Own "Now Playing" session (MediaPlayer framework).
//!
//! Bluetooth AVRCP play/pause/next/previous from the earbuds never become `NX_SYSDEFINED`
//! key events on macOS, so the event tap cannot see them: `mediaremoted` routes them to the
//! current Now Playing app (Music, Spotify, a browser tab), which toggles playback. We become
//! the Now Playing app ourselves (`MPRemoteCommandCenter` handlers + an explicit
//! `MPNowPlayingInfoCenter.playbackState`, which macOS requires instead of inferring it), so
//! the presses reach the dial logic. This is the macOS counterpart of `windows/smtc.rs`.
//!
//! Claiming the state alone loses to any app that plays real audio (a YouTube tab), so we also
//! loop silence through `AVAudioPlayer`, and re-taking the session needs a real state *change*
//! (paused -> fresh info -> silence restarted -> playing): re-announcing the same state is
//! ignored. Approach verified by the Orbit project on a Mac (temp/orbit_ref, input_media.py).

use crate::platform::dial_logic::{now_millis, reported_playing_after, RemoteCommand};
use block2::RcBlock;
use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::OnceLock;

use super::ffi::{objc_getClass, objc_msgSend, sel_registerName};

#[link(name = "MediaPlayer", kind = "framework")]
extern "C" {
    static MPMediaItemPropertyTitle: *mut c_void;
    static MPNowPlayingInfoPropertyElapsedPlaybackTime: *mut c_void;
    static MPNowPlayingInfoPropertyPlaybackRate: *mut c_void;
}

#[link(name = "AVFoundation", kind = "framework")]
extern "C" {}

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
/// Looping silent `AVAudioPlayer` (retained for the life of the app), as a usize for Sync.
static SILENCE_PLAYER: AtomicU64 = AtomicU64::new(0);
static LAST_REASSERT_MS: AtomicU64 = AtomicU64::new(0);
/// Re-taking the session restarts audio; never do it more often than this from rotation.
const REASSERT_THROTTLE_MS: u64 = 2000;
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
        let info = send0(class(b"NSMutableDictionary\0"), b"dictionary\0");
        let set: unsafe extern "C" fn(*mut c_void, *mut c_void, *mut c_void, *mut c_void) =
            std::mem::transmute(objc_msgSend as *const ());
        let number: unsafe extern "C" fn(*mut c_void, *mut c_void, f64) -> *mut c_void =
            std::mem::transmute(objc_msgSend as *const ());
        let num_cls = class(b"NSNumber\0");
        let set_obj = sel(b"setObject:forKey:\0");
        set(info, set_obj, nsstring("Dial Control"), MPMediaItemPropertyTitle);
        set(info, set_obj, number(num_cls, sel(b"numberWithDouble:\0"), 0.0), MPNowPlayingInfoPropertyElapsedPlaybackTime);
        set(info, set_obj, number(num_cls, sel(b"numberWithDouble:\0"), 1.0), MPNowPlayingInfoPropertyPlaybackRate);
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
        start_silence();
        ACTIVE.store(true, Ordering::SeqCst);
        take_back();
    }
    crate::platform::diag::log("now playing: session registered");
}

fn silence_path() -> std::io::Result<std::path::PathBuf> {
    let dir = std::env::temp_dir().join("dial-control");
    std::fs::create_dir_all(&dir)?;
    let path = dir.join("silence.wav");
    if !path.exists() {
        // 1s of 22.05kHz mono 16-bit silence, looped.
        let samples: u32 = 22050;
        let data_len = samples * 2;
        let mut wav = Vec::with_capacity(44 + data_len as usize);
        wav.extend_from_slice(b"RIFF");
        wav.extend_from_slice(&(36 + data_len).to_le_bytes());
        wav.extend_from_slice(b"WAVEfmt ");
        wav.extend_from_slice(&16u32.to_le_bytes());
        wav.extend_from_slice(&1u16.to_le_bytes()); // PCM
        wav.extend_from_slice(&1u16.to_le_bytes()); // mono
        wav.extend_from_slice(&22050u32.to_le_bytes());
        wav.extend_from_slice(&44100u32.to_le_bytes()); // byte rate
        wav.extend_from_slice(&2u16.to_le_bytes()); // block align
        wav.extend_from_slice(&16u16.to_le_bytes());
        wav.extend_from_slice(b"data");
        wav.extend_from_slice(&data_len.to_le_bytes());
        wav.resize(44 + data_len as usize, 0);
        std::fs::write(&path, wav)?;
    }
    Ok(path)
}

/// Starts the looping silent player. Main thread.
unsafe fn start_silence() {
    let Ok(path) = silence_path() else {
        crate::platform::diag::log("now playing: could not write silence.wav");
        return;
    };
    let url = send_ptr(class(b"NSURL\0"), b"fileURLWithPath:\0", nsstring(&path.to_string_lossy()));
    let player = send0(class(b"AVAudioPlayer\0"), b"alloc\0");
    let init: unsafe extern "C" fn(*mut c_void, *mut c_void, *mut c_void, *mut c_void) -> *mut c_void =
        std::mem::transmute(objc_msgSend as *const ());
    let player = init(player, sel(b"initWithContentsOfURL:error:\0"), url, std::ptr::null_mut());
    if player.is_null() {
        crate::platform::diag::log("now playing: silent player failed (presses may go to other apps)");
        return;
    }
    let set_loops: unsafe extern "C" fn(*mut c_void, *mut c_void, isize) =
        std::mem::transmute(objc_msgSend as *const ());
    set_loops(player, sel(b"setNumberOfLoops:\0"), -1);
    let play: unsafe extern "C" fn(*mut c_void, *mut c_void) -> i8 = std::mem::transmute(objc_msgSend as *const ());
    if play(player, sel(b"play\0")) == 0 {
        crate::platform::diag::log("now playing: silent player did not start (no audio output?)");
    }
    SILENCE_PLAYER.store(player as u64, Ordering::SeqCst);
}

/// Takes Now Playing back with a real state change. Main thread.
unsafe fn take_back() {
    let center = send0(class(b"MPNowPlayingInfoCenter\0"), b"defaultCenter\0");
    if center.is_null() {
        return;
    }
    let target = reported_state();
    let opposite = if target == MP_PLAYBACK_PLAYING { MP_PLAYBACK_PAUSED } else { MP_PLAYBACK_PLAYING };
    send_usize(center, b"setPlaybackState:\0", opposite);
    let player = SILENCE_PLAYER.load(Ordering::SeqCst) as *mut c_void;
    if !player.is_null() {
        // A fresh start makes us the most recent player again.
        send0(player, b"stop\0");
        let set_time: unsafe extern "C" fn(*mut c_void, *mut c_void, f64) =
            std::mem::transmute(objc_msgSend as *const ());
        set_time(player, sel(b"setCurrentTime:\0"), 0.0);
        send0(player, b"play\0");
    }
    publish(target);
    LAST_REASSERT_MS.store(now_millis(), Ordering::SeqCst);
}

/// Becomes the Now Playing app again (another player may have taken over). Call when the
/// dial is about to need clicks: HUD open/close, foreground app change.
pub fn reassert(app: &tauri::AppHandle) {
    if ACTIVE.load(Ordering::SeqCst) && now_millis() >= FORWARDING_UNTIL_MS.load(Ordering::SeqCst) {
        let _ = app.run_on_main_thread(|| unsafe { take_back() });
    }
}

/// `reassert`, throttled for frequent triggers such as dial rotation.
pub fn reassert_throttled(app: &tauri::AppHandle) {
    if now_millis().saturating_sub(LAST_REASSERT_MS.load(Ordering::SeqCst)) > REASSERT_THROTTLE_MS {
        LAST_REASSERT_MS.store(now_millis(), Ordering::SeqCst);
        reassert(app);
    }
}

/// Steps aside so a media key from a HUD slice reaches the real player instead of us (media
/// keys go to the Now Playing app). `send` posts the key; we reclaim the session afterwards.
pub fn forward_to_player(app: &tauri::AppHandle, verb: &str, send: impl FnOnce()) {
    if !ACTIVE.load(Ordering::SeqCst) {
        send();
        return;
    }
    if let Some(player) = forward_by_applescript(verb) {
        crate::platform::diag::log(&format!("now playing: {verb} -> {player} (AppleScript)"));
        // The player that just started claims Now Playing; take it back once it has settled.
        std::thread::sleep(std::time::Duration::from_millis(900));
        reassert(app);
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

/// Sends `verb` (playpause / next track / previous track) straight to Spotify or Music when
/// one of them is playing. Apple Events don't go through Now Playing, so this can't loop
/// back to us. Returns the player's name, or None (no scriptable player playing, e.g. a
/// browser tab), in which case the caller falls back to stepping aside.
fn forward_by_applescript(verb: &str) -> Option<&'static str> {
    for player in ["Spotify", "Music"] {
        let script = format!(
            "if application \"{player}\" is running then\n\
             tell application \"{player}\"\n\
             if player state is playing then\n{verb}\nreturn \"ok\"\nend if\n\
             end tell\nend if\nreturn \"\""
        );
        let out = std::process::Command::new("osascript").args(["-e", &script]).output().ok()?;
        if String::from_utf8_lossy(&out.stdout).trim() == "ok" {
            return Some(player);
        }
    }
    None
}
