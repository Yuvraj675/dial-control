use crate::platform::traits::SystemBrightness;
use std::process::Command;

#[cfg(target_os = "macos")]
use super::ffi::*;

type FnGetBrightness = unsafe extern "C" fn(u32, *mut f32) -> i32;
type FnSetBrightness = unsafe extern "C" fn(u32, f32) -> i32;

pub struct MacBrightness;

impl MacBrightness {
    pub fn new() -> Self {
        Self
    }

    #[cfg(target_os = "macos")]
    unsafe fn get_display_services() -> (Option<FnGetBrightness>, Option<FnSetBrightness>) {
        // DisplayServices is a macOS private framework. Loading it dynamically via dlopen
        // prevents linker errors while preserving full Apple Silicon & Intel hardware backlight control.
        let path = b"/System/Library/PrivateFrameworks/DisplayServices.framework/DisplayServices\0";
        let handle = dlopen(path.as_ptr() as *const _, 1); // 1 = RTLD_LAZY
        if handle.is_null() {
            return (None, None);
        }

        let p_get = dlsym(handle, b"DisplayServicesGetBrightness\0".as_ptr() as *const _);
        let p_set = dlsym(handle, b"DisplayServicesSetBrightness\0".as_ptr() as *const _);

        let get_fn: Option<FnGetBrightness> = if !p_get.is_null() {
            Some(std::mem::transmute(p_get))
        } else {
            None
        };

        let set_fn: Option<FnSetBrightness> = if !p_set.is_null() {
            Some(std::mem::transmute(p_set))
        } else {
            None
        };

        (get_fn, set_fn)
    }
}

impl SystemBrightness for MacBrightness {
    fn get_brightness(&self) -> i32 {
        #[cfg(target_os = "macos")]
        unsafe {
            let (get_fn, _) = Self::get_display_services();
            if let Some(get_brightness) = get_fn {
                let display = CGMainDisplayID();
                let mut val: f32 = 0.0;
                if get_brightness(display, &mut val) == 0 {
                    return ((val * 100.0).round() as i32).clamp(0, 100);
                }
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
            let (_, set_fn) = Self::get_display_services();
            if let Some(set_brightness) = set_fn {
                let display = CGMainDisplayID();
                if set_brightness(display, val) == 0 {
                    println!("[macOS Brightness] Set to {}% via DisplayServices (dynamic)", pct);
                    return pct;
                }
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
