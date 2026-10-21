use crate::platform::traits::SystemAudio;
use std::process::Command;

pub struct LinuxAudio;

impl LinuxAudio {
    pub fn new() -> Self {
        Self
    }
}

impl SystemAudio for LinuxAudio {
    fn get_volume(&self) -> i32 {
        // 1. Try WirePlumber (wpctl)
        if let Ok(output) = Command::new("wpctl")
            .args(["get-volume", "@DEFAULT_AUDIO_SINK@"])
            .output()
        {
            let text = String::from_utf8_lossy(&output.stdout);
            for part in text.split_whitespace() {
                if let Ok(val) = part.parse::<f64>() {
                    return ((val * 100.0).round() as i32).clamp(0, 100);
                }
            }
        }

        // 2. Fallback to PulseAudio (pactl)
        if let Ok(output) = Command::new("pactl")
            .args(["get-sink-volume", "@DEFAULT_SINK@"])
            .output()
        {
            let text = String::from_utf8_lossy(&output.stdout);
            for part in text.split_whitespace() {
                if part.ends_with('%') {
                    if let Ok(v) = part.trim_end_matches('%').parse::<i32>() {
                        return v.clamp(0, 100);
                    }
                }
            }
        }

        50
    }

    fn set_volume(&self, pct: i32) -> i32 {
        let pct = pct.clamp(0, 100);
        let frac = format!("{:.2}", (pct as f64) / 100.0);
        let _ = Command::new("wpctl")
            .args(["set-volume", "-l", "1.0", "@DEFAULT_AUDIO_SINK@", &frac])
            .status();
        let _ = Command::new("pactl")
            .args(["set-sink-volume", "@DEFAULT_SINK@", &format!("{}%", pct)])
            .status();
        println!("[Linux Volume] Set to {}% (wpctl: {})", pct, frac);
        pct
    }

    fn toggle_mute(&self) {
        let _ = Command::new("wpctl")
            .args(["set-mute", "@DEFAULT_AUDIO_SINK@", "toggle"])
            .status();
        println!("[Linux Audio] Toggled default audio sink mute via wpctl");
    }
}
