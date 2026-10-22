use crate::platform::traits::SystemBrightness;
use std::process::Command;
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

pub struct WindowsBrightness {
    cached_brightness: Arc<AtomicI32>,
    pending_target: Arc<AtomicI32>,
    is_updating: Arc<AtomicBool>,
}

impl WindowsBrightness {
    pub fn new() -> Self {
        let cached = Arc::new(AtomicI32::new(70));
        let pending = Arc::new(AtomicI32::new(70));
        let updating = Arc::new(AtomicBool::new(false));

        // Query initial hardware brightness asynchronously without visible console window
        let cached_clone = cached.clone();
        thread::spawn(move || {
            let mut cmd = Command::new("powershell");
            #[cfg(target_os = "windows")]
            {
                use std::os::windows::process::CommandExt;
                cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
            }
            if let Ok(output) = cmd
                .args([
                    "-NoProfile",
                    "-NonInteractive",
                    "-Command",
                    "try { (Get-CimInstance -Namespace root/wmi -ClassName WmiMonitorBrightness -ErrorAction Stop).CurrentBrightness } catch { }",
                ])
                .output()
            {
                if let Ok(s) = String::from_utf8(output.stdout) {
                    if let Ok(val) = s.trim().parse::<i32>() {
                        println!("[Windows Brightness] Initial hardware query: {}%", val);
                        cached_clone.store(val, Ordering::SeqCst);
                    }
                }
            }
        });

        Self {
            cached_brightness: cached,
            pending_target: pending,
            is_updating: updating,
        }
    }

    fn apply_hardware_brightness(&self, target: i32) {
        self.pending_target.store(target, Ordering::SeqCst);

        // Debounce: If already updating in background, the worker loop will pick up the latest pending target
        if self.is_updating.swap(true, Ordering::SeqCst) {
            return;
        }

        let is_updating = self.is_updating.clone();
        let pending_target = self.pending_target.clone();

        thread::spawn(move || {
            // Debounce delay so fast scrolling doesn't spam PowerShell
            thread::sleep(Duration::from_millis(200));

            let final_target = pending_target.load(Ordering::SeqCst);
            let ps_script = format!(
                "try {{ (Get-CimInstance -Namespace root/wmi -ClassName WmiMonitorBrightnessMethods -ErrorAction Stop).WmiSetBrightness(1, {}) }} catch {{ }}",
                final_target
            );

            let mut cmd = Command::new("powershell");
            #[cfg(target_os = "windows")]
            {
                use std::os::windows::process::CommandExt;
                cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
            }
            let _ = cmd
                .args(["-NoProfile", "-NonInteractive", "-Command", &ps_script])
                .status();

            is_updating.store(false, Ordering::SeqCst);
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
