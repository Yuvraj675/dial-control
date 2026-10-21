use crate::platform::linux::input::LinuxInput;
use crate::platform::traits::SystemBrightness;
use std::fs;
use std::process::Command;
use std::sync::Arc;

struct BacklightInfo {
    serial: u32,
    connector: String,
    min: i32,
    max: i32,
    value: i32,
}

pub struct LinuxBrightness {
    input: Arc<LinuxInput>,
}

impl LinuxBrightness {
    pub fn new(input: Arc<LinuxInput>) -> Self {
        Self { input }
    }

    fn get_mutter_resources_brightness(&self) -> Option<i32> {
        let out = Command::new("busctl")
            .args([
                "--user",
                "call",
                "org.gnome.Mutter.DisplayConfig",
                "/org/gnome/Mutter/DisplayConfig",
                "org.gnome.Mutter.DisplayConfig",
                "GetResources",
            ])
            .output()
            .ok()?;

        if !out.status.success() {
            return None;
        }

        let text = String::from_utf8_lossy(&out.stdout);
        let parts: Vec<&str> = text.split_whitespace().collect();
        for (i, &p) in parts.iter().enumerate() {
            if p == "\"backlight\"" && i + 2 < parts.len() {
                if let Ok(val) = parts[i + 2].parse::<i32>() {
                    if val >= 0 {
                        return Some(val.clamp(0, 100));
                    }
                }
            }
        }
        None
    }

    fn get_backlight_info(&self) -> Option<BacklightInfo> {
        let out = Command::new("busctl")
            .args([
                "--user",
                "get-property",
                "org.gnome.Mutter.DisplayConfig",
                "/org/gnome/Mutter/DisplayConfig",
                "org.gnome.Mutter.DisplayConfig",
                "Backlight",
            ])
            .output()
            .ok()?;

        if !out.status.success() {
            return None;
        }

        let text = String::from_utf8_lossy(&out.stdout);
        let parts: Vec<&str> = text.split_whitespace().collect();
        let mut serial: u32 = 4;
        let mut connector = "eDP-1".to_string();
        let mut min: Option<i32> = None;
        let mut max: Option<i32> = None;
        let mut value: Option<i32> = None;

        if let Some(s_idx) = parts.iter().position(|&p| p == "(uaa{sv})") {
            if let Some(s) = parts.get(s_idx + 1).and_then(|p| p.parse::<u32>().ok()) {
                serial = s;
            }
        }
        for (i, &p) in parts.iter().enumerate() {
            if p == "\"connector\"" && i + 2 < parts.len() {
                connector = parts[i + 2].trim_matches('"').to_string();
            } else if p == "\"min\"" && i + 2 < parts.len() {
                if let Ok(v) = parts[i + 2].parse::<i32>() {
                    min = Some(v);
                }
            } else if p == "\"max\"" && i + 2 < parts.len() {
                if let Ok(v) = parts[i + 2].parse::<i32>() {
                    max = Some(v);
                }
            } else if p == "\"value\"" && i + 2 < parts.len() {
                if let Ok(v) = parts[i + 2].parse::<i32>() {
                    value = Some(v);
                }
            }
        }

        if let (Some(min), Some(max), Some(value)) = (min, max, value) {
            Some(BacklightInfo {
                serial,
                connector,
                min,
                max,
                value,
            })
        } else {
            None
        }
    }

    fn get_sysfs_backlight(&self) -> Option<i32> {
        let actual_str = fs::read_to_string("/sys/class/backlight/intel_backlight/actual_brightness")
            .or_else(|_| fs::read_to_string("/sys/class/backlight/amdgpu_bl0/actual_brightness"))
            .ok()?;
        let max_str = fs::read_to_string("/sys/class/backlight/intel_backlight/max_brightness")
            .or_else(|_| fs::read_to_string("/sys/class/backlight/amdgpu_bl0/max_brightness"))
            .ok()?;
        let actual: f64 = actual_str.trim().parse().ok()?;
        let max: f64 = max_str.trim().parse().ok()?;
        if max > 0.0 {
            Some(((actual / max) * 100.0).round().clamp(0.0, 100.0) as i32)
        } else {
            None
        }
    }
}

impl SystemBrightness for LinuxBrightness {
    fn get_brightness(&self) -> i32 {
        // 1. GNOME Mutter GetResources (direct native brightness percentage)
        if let Some(pct) = self.get_mutter_resources_brightness() {
            return pct;
        }
        // 2. Mutter Backlight property
        if let Some(b) = self.get_backlight_info() {
            let range = (b.max - b.min).max(1);
            let pct = (((b.value - b.min) as f64 / range as f64) * 100.0).round() as i32;
            return pct.clamp(0, 100);
        }
        // 3. Fallback to sysfs
        if let Some(pct) = self.get_sysfs_backlight() {
            return pct;
        }
        50
    }

    fn set_brightness(&self, pct: i32) -> i32 {
        let pct = pct.clamp(0, 100);
        if let Some(b) = self.get_backlight_info() {
            let range = b.max - b.min;
            let raw = b.min + (pct * range) / 100;
            let _ = Command::new("busctl")
                .args([
                    "--user",
                    "call",
                    "org.gnome.Mutter.DisplayConfig",
                    "/org/gnome/Mutter/DisplayConfig",
                    "org.gnome.Mutter.DisplayConfig",
                    "SetBacklight",
                    "usi",
                    &b.serial.to_string(),
                    &b.connector,
                    &raw.to_string(),
                ])
                .status();
            println!("[Linux Brightness] Set to {}% on {}", pct, b.connector);
        }
        pct
    }

    fn step_brightness(&self, direction: i32) {
        if direction > 0 {
            self.input.emit_brightness_up();
            println!("[Linux Brightness] Emitted KEY_BRIGHTNESSUP (Fn+F6)");
        } else if direction < 0 {
            self.input.emit_brightness_down();
            println!("[Linux Brightness] Emitted KEY_BRIGHTNESSDOWN (Fn+F5)");
        }
    }
}
