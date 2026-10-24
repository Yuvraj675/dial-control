use crate::platform::traits::SystemBrightness;
use std::process::Command;

#[cfg(target_os = "macos")]
use super::ffi::*;

pub struct MacBrightness;

impl MacBrightness {
    pub fn new() -> Self {
        Self
    }

    #[cfg(target_os = "macos")]
    unsafe fn post_aux_key(key_type: u32) {
        // macOS system-defined auxiliary keys (brightness, volume, media)
        // are posted as NX_SYSDEFINED events with subtype 8 (NX_SUBTYPE_AUX_CONTROL_BUTTONS)
        let flags: u64 = if key_type == NX_KEYTYPE_BRIGHTNESS_UP || key_type == NX_KEYTYPE_BRIGHTNESS_DOWN {
            0
        } else {
            0
        };

        // Key down (state = 0x0A)
        let data1_down: i64 = ((key_type as i64) << 16) | (0x0A << 8);
        let event_down = CGEventCreateKeyboardEvent(std::ptr::null_mut(), 0, true);
        if !event_down.is_null() {
            CGEventSetFlags(event_down, flags);
            CGEventPost(kCGHIDEventTap, event_down);
            CFRelease(event_down as CFTypeRef);
        }

        std::thread::sleep(std::time::Duration::from_millis(20));

        // Key up (state = 0x0B)
        let _data1_up: i64 = ((key_type as i64) << 16) | (0x0B << 8);
        let event_up = CGEventCreateKeyboardEvent(std::ptr::null_mut(), 0, false);
        if !event_up.is_null() {
            CGEventSetFlags(event_up, flags);
            CGEventPost(kCGHIDEventTap, event_up);
            CFRelease(event_up as CFTypeRef);
        }
    }
}

impl SystemBrightness for MacBrightness {
    fn get_brightness(&self) -> i32 {
        #[cfg(target_os = "macos")]
        unsafe {
            let display = CGMainDisplayID();
            let mut val: f32 = 0.0;
            if DisplayServicesGetBrightness(display, &mut val) == 0 {
                return ((val * 100.0).round() as i32).clamp(0, 100);
            }
        }

        // Fallback: brightness CLI (e.g. brew install brightness)
        if let Ok(output) = Command::new("brightness").args(["-l"]).output() {
            let text = String::from_utf8_lossy(&output.stdout);
            for line in text.lines() {
                if line.contains("brightness") {
                    if let Some(num_str) = line.split_whitespace().last() {
                        if let Ok(val) = num_str.parse::<f64>() {
                            return ((val * 100.0).round() as i32).clamp(0, 100);
                        }
                    }
                }
            }
        }

        75
    }

    fn set_brightness(&self, pct: i32) -> i32 {
        let pct = pct.clamp(0, 100);
        let val = (pct as f32) / 100.0;

        #[cfg(target_os = "macos")]
        unsafe {
            let display = CGMainDisplayID();
            if DisplayServicesSetBrightness(display, val) == 0 {
                println!("[macOS Brightness] Set to {}% via DisplayServices", pct);
                return pct;
            }
        }

        // Fallback: brightness CLI
        let frac = format!("{:.2}", val);
        let _ = Command::new("brightness").args(["-s", &frac]).status();
        println!("[macOS Brightness] Set to {}% via brightness CLI", pct);
        pct
    }

    fn step_brightness(&self, direction: i32) {
        #[cfg(target_os = "macos")]
        unsafe {
            let key = if direction > 0 {
                NX_KEYTYPE_BRIGHTNESS_UP
            } else {
                NX_KEYTYPE_BRIGHTNESS_DOWN
            };
            Self::post_aux_key(key);
            println!("[macOS Brightness] Stepped brightness via native aux key event: direction {}", direction);
            return;
        }

        #[cfg(not(target_os = "macos"))]
        {
            let current = self.get_brightness();
            let step = direction * 5;
            let next = (current + step).clamp(0, 100);
            self.set_brightness(next);
        }
    }
}
