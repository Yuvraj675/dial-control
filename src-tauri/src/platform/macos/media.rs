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
    unsafe fn post_media_key(key_type: u32) -> bool {
        let cls = objc_getClass(b"NSEvent\0".as_ptr() as *const _);
        if cls.is_null() {
            return false;
        }

        let sel_other = sel_registerName(
            b"otherEventWithType:location:modifierFlags:timestamp:windowNumber:context:subtype:data1:data2:\0"
                .as_ptr() as *const _,
        );
        let sel_cg_event = sel_registerName(b"CGEvent\0".as_ptr() as *const _);

        type MsgSendOtherEvent = unsafe extern "C" fn(
            *mut std::ffi::c_void,
            *mut std::ffi::c_void,
            usize,
            NSPoint,
            usize,
            f64,
            isize,
            *mut std::ffi::c_void,
            i16,
            isize,
            isize,
        ) -> *mut std::ffi::c_void;

        type MsgSendCGEvent =
            unsafe extern "C" fn(*mut std::ffi::c_void, *mut std::ffi::c_void) -> CGEventRef;

        let msg_send_other: MsgSendOtherEvent = std::mem::transmute(objc_msgSend as *const ());
        let msg_send_cg: MsgSendCGEvent = std::mem::transmute(objc_msgSend as *const ());

        // Key Down
        let flags_down = 0xa00;
        let data1_down = ((key_type as isize) << 16) | (0xa << 8);
        let event_down = msg_send_other(
            cls,
            sel_other,
            14, // NSSystemDefined
            NSPoint { x: 0.0, y: 0.0 },
            flags_down,
            0.0,
            0,
            std::ptr::null_mut(),
            8, // NX_SUBTYPE_AUX_CONTROL_BUTTONS
            data1_down,
            -1,
        );

        if !event_down.is_null() {
            let cg_down = msg_send_cg(event_down, sel_cg_event);
            if !cg_down.is_null() {
                CGEventPost(kCGHIDEventTap, cg_down);
            }
        } else {
            return false;
        }

        std::thread::sleep(std::time::Duration::from_millis(20));

        // Key Up
        let flags_up = 0xb00;
        let data1_up = ((key_type as isize) << 16) | (0xb << 8);
        let event_up = msg_send_other(
            cls,
            sel_other,
            14, // NSSystemDefined
            NSPoint { x: 0.0, y: 0.0 },
            flags_up,
            0.0,
            0,
            std::ptr::null_mut(),
            8,
            data1_up,
            -1,
        );

        if !event_up.is_null() {
            let cg_up = msg_send_cg(event_up, sel_cg_event);
            if !cg_up.is_null() {
                CGEventPost(kCGHIDEventTap, cg_up);
            }
        }

        println!("[macOS Media] Dispatched native auxiliary media key 0x{:X}", key_type);
        true
    }
}

impl MediaControl for MacMedia {
    fn play_pause(&self) {
        #[cfg(target_os = "macos")]
        unsafe {
            if Self::post_media_key(NX_KEYTYPE_PLAY) {
                return;
            }
        }

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
            if Self::post_media_key(NX_KEYTYPE_NEXT) {
                return;
            }
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
            if Self::post_media_key(NX_KEYTYPE_PREVIOUS) {
                return;
            }
        }

        let _ = Command::new("osascript")
            .args([
                "-e",
                "if application \"Spotify\" is running then tell application \"Spotify\" to previous track else if application \"Music\" is running then tell application \"Music\" to previous track end if",
            ])
            .status();
    }
}
