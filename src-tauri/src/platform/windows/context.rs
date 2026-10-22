use crate::platform::common::{match_profile, ContextEvent};
use crate::platform::traits::ContextDetector;
use crate::platform::windows::ffi::*;
use std::thread;
use std::time::Duration;
use tauri::{AppHandle, Emitter};

pub struct WindowsContext;

impl WindowsContext {
    pub fn new() -> Self {
        Self
    }
}

impl ContextDetector for WindowsContext {
    fn start(&self, app: AppHandle) {
        thread::spawn(move || {
            let my_pid = unsafe { GetCurrentProcessId() };
            let mut last_profile = String::new();
            let mut last_title = String::new();

            loop {
                thread::sleep(Duration::from_millis(400));

                unsafe {
                    let hwnd = GetForegroundWindow();
                    if hwnd.is_null() {
                        continue;
                    }

                    let mut pid: DWORD = 0;
                    GetWindowThreadProcessId(hwnd, &mut pid);

                    // Skip our own process (HUD, Settings window)
                    if pid == 0 || pid == my_pid {
                        continue;
                    }

                    // 1. Get window title
                    let mut title_buf = [0u16; 512];
                    let title_len = GetWindowTextW(hwnd, title_buf.as_mut_ptr(), 512);
                    let title = if title_len > 0 {
                        String::from_utf16_lossy(&title_buf[..title_len as usize])
                    } else {
                        String::new()
                    };

                    // Ignore Dial Control windows by title
                    if title.contains("Dial Control") {
                        continue;
                    }

                    // 2. Get process executable name
                    let mut app_name = String::new();
                    let h_process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, FALSE, pid);
                    if !h_process.is_null() {
                        let mut exe_buf = [0u16; 1024];
                        let mut size: DWORD = 1024;
                        if QueryFullProcessImageNameW(h_process, 0, exe_buf.as_mut_ptr(), &mut size) != 0 {
                            let full_path = String::from_utf16_lossy(&exe_buf[..size as usize]);
                            if let Some(filename) = full_path.split('\\').last() {
                                // Strip .exe extension for cleaner query
                                app_name = filename
                                    .to_lowercase()
                                    .strip_suffix(".exe")
                                    .unwrap_or(filename)
                                    .to_string();
                            }
                        }
                        CloseHandle(h_process);
                    }

                    if app_name.is_empty() && title.is_empty() {
                        continue;
                    }

                    // 3. Match against profile rules
                    let profile_id = match_profile(&app_name, &app_name, &title);

                    if profile_id != last_profile || title != last_title {
                        last_profile = profile_id.to_string();
                        last_title = title.clone();

                        println!(
                            "[Windows Context] Focus -> App: '{}' | Title: '{}' | Profile: {}",
                            app_name, title, profile_id
                        );

                        let event = ContextEvent {
                            profile_id: profile_id.to_string(),
                            app: app_name.clone(),
                            title: title.clone(),
                            wm_class: app_name.clone(),
                        };
                        let _ = app.emit("context:change", event);
                    }
                }
            }
        });
    }
}
