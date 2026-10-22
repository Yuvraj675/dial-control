use crate::platform::traits::SystemBrightness;
use std::process::Command;
use std::sync::atomic::{AtomicI32, Ordering};
use std::thread;

pub struct WindowsBrightness {
    cached_brightness: AtomicI32,
}

impl WindowsBrightness {
    pub fn new() -> Self {
        let instance = Self {
            cached_brightness: AtomicI32::new(70),
        };

        // Query initial hardware brightness asynchronously in background
        thread::spawn(|| {
            if let Ok(output) = Command::new("powershell")
                .args([
                    "-NoProfile",
                    "-NonInteractive",
                    "-Command",
                    "(Get-CimInstance -Namespace root/wmi -ClassName WmiMonitorBrightness).CurrentBrightness",
                ])
                .output()
            {
                if let Ok(s) = String::from_utf8(output.stdout) {
                    if let Ok(val) = s.trim().parse::<i32>() {
                        println!("[Windows Brightness] Initial hardware query: {}%", val);
                    }
                }
            }
        });

        instance
    }

    fn apply_hardware_brightness(&self, target: i32) {
        thread::spawn(move || {
            let cmd = format!(
                "(Get-CimInstance -Namespace root/wmi -ClassName WmiMonitorBrightnessMethods).WmiSetBrightness(1, {})",
                target
            );
            let _ = Command::new("powershell")
                .args(["-NoProfile", "-NonInteractive", "-Command", &cmd])
                .status();
        });
    }
}

impl SystemBrightness for WindowsBrightness {
    fn get_brightness(&self) -> i32 {
        self.cached_brightness.load(Ordering::SeqCst)
    }

    fn set_brightness(&self, pct: i32) -> i32 {
        let target = pct.clamp(0, 100);
        self.cached_brightness.store(target, Ordering::SeqCst);
        self.apply_hardware_brightness(target);
        println!("[Windows Brightness] Brightness set to {}%", target);
        target
    }

    fn step_brightness(&self, direction: i32) {
        let current = self.get_brightness();
        let step = if direction > 0 { 5 } else { -5 };
        let new_val = (current + step).clamp(0, 100);
        self.set_brightness(new_val);
    }
}
