use crate::platform::dial_logic::{brightness_cache_is_fresh, brightness_step_target, now_millis};
use crate::platform::traits::SystemBrightness;
use crate::platform::windows::ffi::*;
use std::process::Command;
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

unsafe extern "system" fn enum_mon_proc(
    hmon: HMONITOR,
    _hdc: HDC,
    _rect: *mut std::ffi::c_void,
    lparam: LPARAM,
) -> BOOL {
    let monitors = &mut *(lparam as *mut Vec<HMONITOR>);
    monitors.push(hmon);
    1
}

fn query_dxva2_brightness() -> Option<i32> {
    unsafe {
        let dxva2_name: Vec<u16> = "dxva2.dll\0".encode_utf16().collect();
        let h_dxva2 = LoadLibraryW(dxva2_name.as_ptr());
        if h_dxva2.is_null() {
            return None;
        }

        let p_get_num = GetProcAddress(h_dxva2, b"GetNumberOfPhysicalMonitorsFromHMONITOR\0".as_ptr());
        let p_get_phys = GetProcAddress(h_dxva2, b"GetPhysicalMonitorsFromHMONITOR\0".as_ptr());
        let p_destroy = GetProcAddress(h_dxva2, b"DestroyPhysicalMonitors\0".as_ptr());
        let p_get_bright = GetProcAddress(h_dxva2, b"GetMonitorBrightness\0".as_ptr());

        if p_get_num.is_null()
            || p_get_phys.is_null()
            || p_destroy.is_null()
            || p_get_bright.is_null()
        {
            FreeLibrary(h_dxva2);
            return None;
        }

        let get_num: FnGetNumberOfPhysicalMonitorsFromHMONITOR = std::mem::transmute(p_get_num);
        let get_phys: FnGetPhysicalMonitorsFromHMONITOR = std::mem::transmute(p_get_phys);
        let destroy: FnDestroyPhysicalMonitors = std::mem::transmute(p_destroy);
        let get_bright: FnGetMonitorBrightness = std::mem::transmute(p_get_bright);

        let mut monitors: Vec<HMONITOR> = Vec::new();
        let _ = EnumDisplayMonitors(
            std::ptr::null_mut(),
            std::ptr::null(),
            Some(enum_mon_proc),
            &mut monitors as *mut _ as LPARAM,
        );

        let mut found_brightness: Option<i32> = None;
        for hmon in monitors {
            let mut count: DWORD = 0;
            if get_num(hmon, &mut count) != 0 && count > 0 {
                let mut phys = vec![
                    PHYSICAL_MONITOR {
                        hPhysicalMonitor: std::ptr::null_mut(),
                        szPhysicalMonitorDescription: [0; 128],
                    };
                    count as usize
                ];
                if get_phys(hmon, count, phys.as_mut_ptr()) != 0 {
                    for p in &phys {
                        let mut min: DWORD = 0;
                        let mut cur: DWORD = 0;
                        let mut max: DWORD = 100;
                        if get_bright(p.hPhysicalMonitor, &mut min, &mut cur, &mut max) != 0 {
                            if max > min {
                                let span = max as i32 - min as i32;
                                let pct = (((cur as i32 - min as i32) * 100) / span).clamp(0, 100);
                                found_brightness = Some(pct);
                                break;
                            }
                        }
                    }
                    let _ = destroy(count, phys.as_mut_ptr());
                }
            }
            if found_brightness.is_some() {
                break;
            }
        }

        FreeLibrary(h_dxva2);
        found_brightness
    }
}

fn set_dxva2_brightness(pct: i32) {
    unsafe {
        let dxva2_name: Vec<u16> = "dxva2.dll\0".encode_utf16().collect();
        let h_dxva2 = LoadLibraryW(dxva2_name.as_ptr());
        if h_dxva2.is_null() {
            return;
        }

        let p_get_num = GetProcAddress(h_dxva2, b"GetNumberOfPhysicalMonitorsFromHMONITOR\0".as_ptr());
        let p_get_phys = GetProcAddress(h_dxva2, b"GetPhysicalMonitorsFromHMONITOR\0".as_ptr());
        let p_destroy = GetProcAddress(h_dxva2, b"DestroyPhysicalMonitors\0".as_ptr());
        let p_get_bright = GetProcAddress(h_dxva2, b"GetMonitorBrightness\0".as_ptr());
        let p_set_bright = GetProcAddress(h_dxva2, b"SetMonitorBrightness\0".as_ptr());

        if p_get_num.is_null()
            || p_get_phys.is_null()
            || p_destroy.is_null()
            || p_get_bright.is_null()
            || p_set_bright.is_null()
        {
            FreeLibrary(h_dxva2);
            return;
        }

        let get_num: FnGetNumberOfPhysicalMonitorsFromHMONITOR = std::mem::transmute(p_get_num);
        let get_phys: FnGetPhysicalMonitorsFromHMONITOR = std::mem::transmute(p_get_phys);
        let destroy: FnDestroyPhysicalMonitors = std::mem::transmute(p_destroy);
        let get_bright: FnGetMonitorBrightness = std::mem::transmute(p_get_bright);
        let set_bright: FnSetMonitorBrightness = std::mem::transmute(p_set_bright);

        let mut monitors: Vec<HMONITOR> = Vec::new();
        let _ = EnumDisplayMonitors(
            std::ptr::null_mut(),
            std::ptr::null(),
            Some(enum_mon_proc),
            &mut monitors as *mut _ as LPARAM,
        );

        for hmon in monitors {
            let mut count: DWORD = 0;
            if get_num(hmon, &mut count) != 0 && count > 0 {
                let mut phys = vec![
                    PHYSICAL_MONITOR {
                        hPhysicalMonitor: std::ptr::null_mut(),
                        szPhysicalMonitorDescription: [0; 128],
                    };
                    count as usize
                ];
                if get_phys(hmon, count, phys.as_mut_ptr()) != 0 {
                    for p in &phys {
                        let mut min: DWORD = 0;
                        let mut cur: DWORD = 0;
                        let mut max: DWORD = 100;
                        if get_bright(p.hPhysicalMonitor, &mut min, &mut cur, &mut max) != 0 {
                            let span = (max as i32 - min as i32).max(1);
                            let mapped = (min as i32 + (pct * span) / 100).clamp(min as i32, max as i32) as DWORD;
                            let _ = set_bright(p.hPhysicalMonitor, mapped);
                        }
                    }
                    let _ = destroy(count, phys.as_mut_ptr());
                }
            }
        }

        FreeLibrary(h_dxva2);
    }
}

fn query_wmi_brightness() -> Option<i32> {
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
            "$b = (Get-CimInstance -Namespace root/wmi -ClassName WmiMonitorBrightness -ErrorAction SilentlyContinue).CurrentBrightness; if ($null -eq $b) { $b = (Get-WmiObject -Namespace root/wmi -ClassName WmiMonitorBrightness -ErrorAction SilentlyContinue).CurrentBrightness }; if ($null -ne $b) { Write-Output $b }",
        ])
        .output()
    {
        if let Ok(s) = String::from_utf8(output.stdout) {
            if let Ok(val) = s.trim().parse::<i32>() {
                return Some(val.clamp(0, 100));
            }
        }
    }
    None
}

pub struct WindowsBrightness {
    cached_brightness: Arc<AtomicI32>,
    pending_target: Arc<AtomicI32>,
    is_updating: Arc<AtomicBool>,
    last_set_time: Arc<AtomicUsize>,
}

impl WindowsBrightness {
    pub fn new() -> Self {
        let cached = Arc::new(AtomicI32::new(70));
        let pending = Arc::new(AtomicI32::new(70));
        let updating = Arc::new(AtomicBool::new(false));
        let last_set = Arc::new(AtomicUsize::new(0));

        let cached_poll = cached.clone();

        // One-time startup probe in background thread to prime initial hardware brightness
        thread::spawn(move || {
            if let Some(val) = query_dxva2_brightness().or_else(query_wmi_brightness) {
                println!("[Windows Brightness] Initial hardware brightness detected: {}%", val);
                cached_poll.store(val, Ordering::SeqCst);
            }
        });

        Self {
            cached_brightness: cached,
            pending_target: pending,
            is_updating: updating,
            last_set_time: last_set,
        }
    }

    fn apply_hardware_brightness(&self, target: i32) {
        self.pending_target.store(target, Ordering::SeqCst);

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as usize;
        self.last_set_time.store(now, Ordering::SeqCst);

        // Adjust external desktop monitors (DDC/CI via dxva2) immediately (< 1ms)
        set_dxva2_brightness(target);

        // Debounce: If already updating in background, the worker loop will pick up the latest pending target
        if self.is_updating.swap(true, Ordering::SeqCst) {
            return;
        }

        let is_updating = self.is_updating.clone();
        let pending_target = self.pending_target.clone();

        thread::spawn(move || {
            // Debounce delay so fast rotary ticks don't spam PowerShell
            thread::sleep(Duration::from_millis(150));

            let final_target = pending_target.load(Ordering::SeqCst);

            // Execute both WmiObject and Invoke-CimMethod to guarantee compatibility across all Windows 10 & 11 releases
            let ps_script = format!(
                "try {{ (Get-WmiObject -Namespace root/wmi -ClassName WmiMonitorBrightnessMethods -ErrorAction SilentlyContinue).WmiSetBrightness(1, {0}) }} catch {{ }}; try {{ $m = Get-CimInstance -Namespace root/wmi -ClassName WmiMonitorBrightnessMethods -ErrorAction SilentlyContinue; if ($m) {{ Invoke-CimMethod -InputObject $m -MethodName WmiSetBrightness -Arguments @{{Brightness = [byte]{0}; Timeout = [uint32]1}} -ErrorAction SilentlyContinue }} }} catch {{ }}",
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
        // Query live hardware brightness: first check external monitors via DDC/CI (< 1ms),
        // then fall back to internal laptop screens via WMI.
        if let Some(val) = query_dxva2_brightness().or_else(query_wmi_brightness) {
            self.cached_brightness.store(val, Ordering::SeqCst);
            return val;
        }
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
        // WMI applies brightness asynchronously (150ms debounce + PowerShell), so right after
        // our own set a hardware read still returns the old level and fast rotation would
        // stall at +5%. Step from our own target until the hardware has settled.
        let last_set = self.last_set_time.load(Ordering::SeqCst) as u64;
        let current = if brightness_cache_is_fresh(now_millis(), last_set) {
            self.pending_target.load(Ordering::SeqCst)
        } else {
            self.get_brightness()
        };
        self.set_brightness(brightness_step_target(current, direction));
    }
}
