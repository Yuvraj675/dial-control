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
