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

    #[cfg(target_os = "macos")]
    fn key_name_to_keycode(name: &str) -> Option<u16> {
        let clean = name.trim().to_uppercase();
        match clean.as_str() {
            "A" => Some(0x00),
            "S" => Some(0x01),
            "D" => Some(0x02),
            "F" => Some(0x03),
            "H" => Some(0x04),
            "G" => Some(0x05),
            "Z" => Some(0x06),
            "X" => Some(0x07),
            "C" => Some(0x08),
            "V" => Some(0x09),
            "B" => Some(0x0B),
            "Q" => Some(0x0C),
            "W" => Some(0x0D),
            "E" => Some(0x0E),
            "R" => Some(0x0F),
            "Y" => Some(0x10),
            "T" => Some(0x11),
            "1" => Some(0x12),
            "2" => Some(0x13),
            "3" => Some(0x14),
            "4" => Some(0x15),
            "6" => Some(0x16),
            "5" => Some(0x17),
            "EQUAL" | "=" => Some(0x18),
            "9" => Some(0x19),
            "7" => Some(0x1A),
            "MINUS" | "-" => Some(0x1B),
            "8" => Some(0x1C),
            "0" => Some(0x1D),
            "O" => Some(0x1F),
            "U" => Some(0x20),
            "I" => Some(0x22),
            "P" => Some(0x23),
            "L" => Some(0x25),
            "J" => Some(0x26),
            "K" => Some(0x28),
            "SEMICOLON" | ";" => Some(0x29),
            "N" => Some(0x2D),
            "M" => Some(0x2E),
            "COMMA" | "," => Some(0x2B),
            "PERIOD" | "." => Some(0x2F),
            "SLASH" | "/" => Some(0x2C),
            "BACKSLASH" | "\\" => Some(0x2A),
            "GRAVE" | "`" => Some(0x32),
            "RETURN" | "ENTER" => Some(0x24),
            "TAB" => Some(0x30),
            "SPACE" => Some(0x31),
            "DELETE" | "BACKSPACE" => Some(0x33),
            "ESCAPE" | "ESC" => Some(0x35),
            "F1" => Some(0x7A),
            "F2" => Some(0x78),
            "F3" => Some(0x63),
            "F4" => Some(0x76),
            "F5" => Some(0x60),
            "F6" => Some(0x61),
            "F7" => Some(0x62),
            "F8" => Some(0x64),
            "F9" => Some(0x65),
            "F10" => Some(0x6D),
            "F11" => Some(0x67),
            "F12" => Some(0x6F),
            "LEFT" | "ARROWLEFT" => Some(0x7B),
            "RIGHT" | "ARROWRIGHT" => Some(0x7C),
            "DOWN" | "ARROWDOWN" => Some(0x7D),
            "UP" | "ARROWUP" => Some(0x7E),
            _ => None,
        }
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

    fn emit_hotkey(&self, keys: &[String]) {
        #[cfg(target_os = "macos")]
        unsafe {
            let mut modifier_flags: u64 = 0;
            let mut target_keycode: Option<u16> = None;

            for key in keys {
                let upper = key.trim().to_uppercase();
                match upper.as_str() {
                    "SUPER" | "COMMAND" | "CMD" | "META" => {
                        modifier_flags |= kCGEventFlagMaskCommand;
                    }
                    "SHIFT" => {
                        modifier_flags |= kCGEventFlagMaskShift;
                    }
                    "CTRL" | "CONTROL" => {
                        modifier_flags |= kCGEventFlagMaskControl;
                    }
                    "ALT" | "OPTION" | "OPT" => {
                        modifier_flags |= kCGEventFlagMaskAlternate;
                    }
                    _ => {
                        if let Some(code) = Self::key_name_to_keycode(&upper) {
                            target_keycode = Some(code);
                        }
                    }
                }
            }

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
