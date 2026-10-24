use crate::platform::traits::SystemBrightness;
use std::process::Command;

#[cfg(target_os = "macos")]
use super::ffi::*;

pub struct MacBrightness;

impl MacBrightness {
    pub fn new() -> Self {
        Self
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
        let current = self.get_brightness();
        let step = direction * 5;
        let next = (current + step).clamp(0, 100);
        self.set_brightness(next);
        println!("[macOS Brightness] Stepped brightness: {}% -> {}%", current, next);
    }
}
