use crate::platform::traits::VirtualInput;
use std::thread::sleep;
use std::time::Duration;

#[cfg(target_os = "macos")]
use super::ffi::*;

pub struct MacInput;

impl MacInput {
    pub fn new() -> Self {
        Self
    }
}

impl VirtualInput for MacInput {
    fn emit_scroll(&self, delta: i32) {
        #[cfg(target_os = "macos")]
        unsafe {
            // kCGScrollEventUnitLine = 1. delta > 0 is scroll up, delta < 0 is scroll down
            let event = CGEventCreateScrollWheelEvent2(
                std::ptr::null_mut(),
                kCGScrollEventUnitLine,
                1,
                delta,
                0,
                0,
            );

            if !event.is_null() {
                CGEventPost(kCGHIDEventTap, event);
                CFRelease(event as CFTypeRef);
            }
        }

        #[cfg(not(target_os = "macos"))]
        {
            println!("[macOS Mock Input] emit_scroll: {}", delta);
        }
    }

    fn type_text(&self, text: &str) {
        #[cfg(target_os = "macos")]
        unsafe {
            // One event per UTF-16 chunk; the keycode is ignored when a Unicode string is set.
            let units: Vec<u16> = text.encode_utf16().collect();
            for chunk in units.chunks(20) {
                for down in [true, false] {
                    let event = CGEventCreateKeyboardEvent(std::ptr::null_mut(), 0, down);
                    if !event.is_null() {
                        CGEventKeyboardSetUnicodeString(event, chunk.len() as _, chunk.as_ptr());
                        CGEventPost(kCGHIDEventTap, event);
                        CFRelease(event as CFTypeRef);
                    }
                }
                sleep(Duration::from_millis(8));
            }
        }

        #[cfg(not(target_os = "macos"))]
        {
            println!("[macOS Mock Input] type_text: {:?}", text);
        }
    }

    fn emit_native_hotkey(&self, keys: &[String]) {
        self.post_hotkey(crate::platform::keymap::mac_hotkey_literal(keys), keys);
    }

    fn emit_hotkey(&self, keys: &[String]) {
        // Shared PC-style shortcut: translate Ctrl to Command; see keymap::mac_hotkey.
        self.post_hotkey(crate::platform::keymap::mac_hotkey(keys), keys);
    }
}

impl MacInput {
    fn post_hotkey(&self, resolved: Option<(u64, u16)>, keys: &[String]) {
        #[cfg(target_os = "macos")]
        unsafe {
            if resolved.is_none() {
                eprintln!("[macOS Input] Could not resolve hotkey {:?}", keys);
            }
            let modifier_flags = resolved.map(|(flags, _)| flags).unwrap_or(0);
            let target_keycode = resolved.map(|(_, code)| code);

            if let Some(code) = target_keycode {
                // Key down
                let event_down = CGEventCreateKeyboardEvent(std::ptr::null_mut(), code, true);
                if !event_down.is_null() {
                    CGEventSetFlags(event_down, modifier_flags);
                    CGEventPost(kCGHIDEventTap, event_down);
                    CFRelease(event_down as CFTypeRef);
                }

                sleep(Duration::from_millis(25));

                // Key up
                let event_up = CGEventCreateKeyboardEvent(std::ptr::null_mut(), code, false);
                if !event_up.is_null() {
                    CGEventSetFlags(event_up, modifier_flags);
                    CGEventPost(kCGHIDEventTap, event_up);
                    CFRelease(event_up as CFTypeRef);
                }

                println!(
                    "[macOS Input] Emitted hotkey keycode 0x{:02X} with flags 0x{:X}",
                    code, modifier_flags
                );
            }
        }

        #[cfg(not(target_os = "macos"))]
        {
            println!("[macOS Mock Input] emit_hotkey: {:?}", keys);
        }
    }
}
