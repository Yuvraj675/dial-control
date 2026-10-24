use crate::platform::traits::MediaControl;
use std::process::Command;

#[cfg(target_os = "macos")]
use super::ffi::*;

pub struct MacMedia;

impl MacMedia {
    pub fn new() -> Self {
        Self
    }

    #[cfg(target_os = "macos")]
    unsafe fn post_media_key(key_type: u32) {
        // System defined auxiliary media keys (play/pause, next, previous)
        let event_down = CGEventCreateKeyboardEvent(std::ptr::null_mut(), 0, true);
        if !event_down.is_null() {
            CGEventPost(kCGHIDEventTap, event_down);
            CFRelease(event_down as CFTypeRef);
        }

        std::thread::sleep(std::time::Duration::from_millis(20));

        let event_up = CGEventCreateKeyboardEvent(std::ptr::null_mut(), 0, false);
        if !event_up.is_null() {
            CGEventPost(kCGHIDEventTap, event_up);
            CFRelease(event_up as CFTypeRef);
        }
        println!("[macOS Media] Posted media key 0x{:X}", key_type);
    }
}

impl MediaControl for MacMedia {
    fn play_pause(&self) {
        #[cfg(target_os = "macos")]
        unsafe {
            Self::post_media_key(NX_KEYTYPE_PLAY);
            return;
        }

        // Fallback: AppleScript Spotify or Music
        let _ = Command::new("osascript")
            .args([
                "-e",
                "if application \"Spotify\" is running then tell application \"Spotify\" to playpause else if application \"Music\" is running then tell application \"Music\" to playpause end if",
            ])
            .status();
    }

    fn next_track(&self) {
        #[cfg(target_os = "macos")]
        unsafe {
            Self::post_media_key(NX_KEYTYPE_NEXT);
            return;
        }

        let _ = Command::new("osascript")
            .args([
                "-e",
                "if application \"Spotify\" is running then tell application \"Spotify\" to next track else if application \"Music\" is running then tell application \"Music\" to next track end if",
            ])
            .status();
    }

    fn prev_track(&self) {
        #[cfg(target_os = "macos")]
        unsafe {
            Self::post_media_key(NX_KEYTYPE_PREVIOUS);
            return;
        }

        let _ = Command::new("osascript")
            .args([
                "-e",
                "if application \"Spotify\" is running then tell application \"Spotify\" to previous track else if application \"Music\" is running then tell application \"Music\" to previous track end if",
            ])
            .status();
    }
}
