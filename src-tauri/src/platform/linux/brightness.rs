use crate::platform::linux::input::LinuxInput;
use crate::platform::traits::SystemBrightness;
use std::fs;
use std::process::Command;
use std::sync::Arc;

#[derive(Debug, Clone)]
struct BacklightInfo {
    _serial: u32,
    _connector: String,
    min: i32,
    max: i32,
    value: i32,
}

fn parse_mutter_busctl(text: &str) -> Option<BacklightInfo> {
    let parts: Vec<&str> = text.split_whitespace().collect();
    let mut serial: u32 = 0;
    let mut connector = String::new();
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
        if max > min {
            return Some(BacklightInfo {
                _serial: serial,
                _connector: connector,
                min,
                max,
                value,
            });
        }
    }
    None
}

fn parse_mutter_gdbus(text: &str) -> Option<BacklightInfo> {
    let serial = if let Some(idx) = text.find("uint32 ") {
        let rest = &text[idx + 7..];
        rest.split(|c: char| !c.is_ascii_digit()).next().and_then(|s| s.parse::<u32>().ok())
    } else {
        None
    }?;

    let connector = if let Some(idx) = text.find("'connector': <'") {
        let rest = &text[idx + 15..];
        rest.split('\'').next().map(|s| s.to_string())
    } else {
        None
    }.unwrap_or_else(|| "eDP-1".to_string());

    let extract_int = |key: &str| -> Option<i32> {
        let idx = text.find(key)?;
        let rest = &text[idx + key.len()..];
        let num_str = rest.trim_start().split(|c: char| !c.is_ascii_digit() && c != '-').next()?;
        num_str.parse::<i32>().ok()
    };

    let min = extract_int("'min': <")?;
    let max = extract_int("'max': <")?;
    let value = extract_int("'value': <")?;

    if max > min {
        Some(BacklightInfo {
            _serial: serial,
            _connector: connector,
            min,
            max,
            value,
        })
    } else {
        None
    }
}

pub struct LinuxBrightness {
    input: Arc<LinuxInput>,
}

impl LinuxBrightness {
    pub fn new(input: Arc<LinuxInput>) -> Self {
        Self { input }
    }

    /// Query GNOME Mutter Backlight property via busctl or gdbus
    fn get_backlight_info(&self) -> Option<BacklightInfo> {
        // 1. Try busctl
        if let Ok(out) = Command::new("busctl")
            .args([
                "--user",
                "get-property",
                "org.gnome.Mutter.DisplayConfig",
                "/org/gnome/Mutter/DisplayConfig",
                "org.gnome.Mutter.DisplayConfig",
                "Backlight",
            ])
            .output()
        {
            if out.status.success() {
                let text = String::from_utf8_lossy(&out.stdout);
                if let Some(info) = parse_mutter_busctl(&text) {
                    return Some(info);
                }
            }
        }

        // 2. Try gdbus fallback
        if let Ok(out) = Command::new("gdbus")
            .args([
                "call",
                "--session",
                "--dest",
                "org.gnome.Mutter.DisplayConfig",
                "--object-path",
                "/org/gnome/Mutter/DisplayConfig",
                "--method",
                "org.freedesktop.DBus.Properties.Get",
                "org.gnome.Mutter.DisplayConfig",
                "Backlight",
            ])
            .output()
        {
            if out.status.success() {
                let text = String::from_utf8_lossy(&out.stdout);
                if let Some(info) = parse_mutter_gdbus(&text) {
                    return Some(info);
                }
            }
        }

        None
    }

    /// Query KDE Plasma PowerDevil D-Bus
    fn get_kde_brightness(&self) -> Option<i32> {
        let cur_out = Command::new("busctl")
            .args([
                "--user",
                "get-property",
                "org.kde.Solid.PowerManagement",
                "/org/kde/Solid/PowerManagement/Actions/BrightnessControl",
                "org.kde.Solid.PowerManagement.Actions.BrightnessControl",
                "brightness",
            ])
            .output()
            .ok()?;

        if !cur_out.status.success() {
            return None;
        }

        let cur_text = String::from_utf8_lossy(&cur_out.stdout);
        let cur_val = cur_text.split_whitespace().last()?.parse::<f64>().ok()?;

        // Query max brightness to compute accurate ratio
        let max_val = if let Ok(max_out) = Command::new("busctl")
            .args([
                "--user",
                "get-property",
                "org.kde.Solid.PowerManagement",
                "/org/kde/Solid/PowerManagement/Actions/BrightnessControl",
                "org.kde.Solid.PowerManagement.Actions.BrightnessControl",
                "brightnessMax",
            ])
            .output()
        {
            let max_text = String::from_utf8_lossy(&max_out.stdout);
            max_text.split_whitespace().last().and_then(|s| s.parse::<f64>().ok()).unwrap_or(100.0)
        } else {
            100.0
        };

        if max_val > 0.0 {
            Some(((cur_val / max_val) * 100.0).round().clamp(0.0, 100.0) as i32)
        } else {
            Some((cur_val as i32).clamp(0, 100))
        }
    }

    /// Query standard Linux brightnessctl utility (used on Arch, Hyprland, Sway, KDE, etc.)
    fn get_brightnessctl(&self) -> Option<i32> {
        let out = Command::new("brightnessctl").args(["-m"]).output().ok()?;
        if !out.status.success() {
            return None;
        }
        let text = String::from_utf8_lossy(&out.stdout);
        for line in text.lines() {
            let parts: Vec<&str> = line.split(',').collect();
            if parts.len() >= 4 {
                let pct_str = parts[3].trim().trim_end_matches('%');
                if let Ok(pct) = pct_str.parse::<i32>() {
                    return Some(pct.clamp(0, 100));
                }
            }
        }
        None
    }

    /// Dynamic sysfs scanner: fallback for minimal Linux kernels / X11 without desktop environment
    fn get_sysfs_backlight(&self) -> Option<i32> {
        let entries = fs::read_dir("/sys/class/backlight").ok()?;
        for entry in entries.flatten() {
            let path = entry.path();
            let cur_str = fs::read_to_string(path.join("actual_brightness"))
                .or_else(|_| fs::read_to_string(path.join("brightness")));
            let max_str = fs::read_to_string(path.join("max_brightness"));

            if let (Ok(cur), Ok(max)) = (cur_str, max_str) {
                if let (Ok(cur_val), Ok(max_val)) = (
                    cur.trim().parse::<f64>(),
                    max.trim().parse::<f64>(),
                ) {
                    if max_val > 0.0 {
                        let pct = ((cur_val / max_val) * 100.0).round().clamp(0.0, 100.0) as i32;
                        return Some(pct);
                    }
                }
            }
        }
        None
    }

    /// Query GNOME Mutter GetResources (direct live backlight percentage, works on OLED & LCD)
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
        if let Some(idx) = text.find("\"backlight\"") {
            let rest = &text[idx + 11..];
            for token in rest.split_whitespace().take(5) {
                if let Ok(val) = token.parse::<i32>() {
                    if (0..=100).contains(&val) {
                        return Some(val);
                    }
                }
            }
        }
        None
    }
}

impl SystemBrightness for LinuxBrightness {
    fn get_brightness(&self) -> i32 {
        // 1. GNOME Mutter GetResources (canonical live percentage on GNOME Wayland & X11, accurate for OLED & LCD)
        if let Some(pct) = self.get_mutter_resources_brightness() {
            return pct;
        }

        // 2. GNOME Mutter DisplayConfig Backlight property fallback
        if let Some(b) = self.get_backlight_info() {
            let range = (b.max - b.min).max(1);
            let pct = (((b.value - b.min) as f64 / range as f64) * 100.0).round() as i32;
            return pct.clamp(0, 100);
        }

        // 3. KDE Plasma PowerDevil D-Bus
        if let Some(pct) = self.get_kde_brightness() {
            return pct;
        }

        // 4. brightnessctl (standard on Arch Linux, Hyprland, Sway, etc.)
        if let Some(pct) = self.get_brightnessctl() {
            return pct;
        }

        // 5. Dynamic sysfs scanner: hardware fallback for non-DE setups
        if let Some(pct) = self.get_sysfs_backlight() {
            return pct;
        }

        50
    }

    fn set_brightness(&self, pct: i32) -> i32 {
        let pct = pct.clamp(0, 100);

        // 1. Try brightnessctl
        let _ = Command::new("brightnessctl")
            .args(["set", &format!("{}%", pct)])
            .output();

        // 2. Try KDE PowerDevil D-Bus
        let _ = Command::new("busctl")
            .args([
                "--user",
                "call",
                "org.kde.Solid.PowerManagement",
                "/org/kde/Solid/PowerManagement/Actions/BrightnessControl",
                "org.kde.Solid.PowerManagement.Actions.BrightnessControl",
                "setBrightness",
                "i",
                &pct.to_string(),
            ])
            .output();

        pct
    }

    fn step_brightness(&self, direction: i32) {
        let mut handled = false;

        // Try brightnessctl first
        if direction > 0 {
            if let Ok(status) = Command::new("brightnessctl").args(["set", "+5%"]).status() {
                if status.success() {
                    handled = true;
                }
            }
        } else if direction < 0 {
            if let Ok(status) = Command::new("brightnessctl").args(["set", "5%-"]).status() {
                if status.success() {
                    handled = true;
                }
            }
        }

        // Fallback to uinput virtual keyboard brightness keys (emits hardware Fn+F5 / Fn+F6 keys to GNOME)
        if !handled {
            if direction > 0 {
                self.input.emit_brightness_up();
                println!("[Linux Brightness] Emitted KEY_BRIGHTNESSUP (Fn+F6) through GNOME");
            } else if direction < 0 {
                self.input.emit_brightness_down();
                println!("[Linux Brightness] Emitted KEY_BRIGHTNESSDOWN (Fn+F5) through GNOME");
            }
        }
    }
}

