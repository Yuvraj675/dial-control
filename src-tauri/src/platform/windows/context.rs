use crate::platform::common::{match_profile, ContextEvent};
use crate::platform::traits::ContextDetector;
use crate::platform::windows::ffi::*;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;
use std::time::Duration;
use tauri::{AppHandle, Emitter};

static LAST_TARGET_HWND: AtomicUsize = AtomicUsize::new(0);

pub fn get_last_target_hwnd() -> HWND {
    LAST_TARGET_HWND.load(Ordering::SeqCst) as HWND
}

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
                thread::sleep(Duration::from_millis(350));

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

                    // Record target window for hotkey focus restoration
                    LAST_TARGET_HWND.store(hwnd as usize, Ordering::SeqCst);

                    // 2. Get window class name (works 100% without process privileges)
                    let mut class_buf = [0u16; 256];
                    let class_len = GetClassNameW(hwnd, class_buf.as_mut_ptr(), 256);
                    let class_name = if class_len > 0 {
                        String::from_utf16_lossy(&class_buf[..class_len as usize])
                    } else {
                        String::new()
                    };

                    // 3. Get process executable name
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

                    // Fallback heuristics using class name if process path was unreadable
                    if app_name.is_empty() {
                        if class_name.contains("Chrome_WidgetWin") {
                            app_name = "chrome".to_string();
                        } else if class_name.contains("MozillaWindowClass") {
                            app_name = "firefox".to_string();
                        } else if class_name == "CabinetWClass" || class_name == "ExploreWClass" {
                            app_name = "explorer".to_string();
                        }
                    }

                    if app_name.is_empty() && title.is_empty() && class_name.is_empty() {
                        continue;
                    }

                    // 4. Match against profile rules
                    let profile_id = match_profile(&app_name, &class_name, &title);

                    if profile_id != last_profile || title != last_title {
                        last_profile = profile_id.to_string();
                        last_title = title.clone();

                        println!(
                            "[Windows Context] Focus -> App: '{}' | Class: '{}' | Title: '{}' | Profile: {}",
                            app_name, class_name, title, profile_id
                        );

                        let event = ContextEvent {
                            profile_id: profile_id.to_string(),
                            app: if !app_name.is_empty() { app_name } else { class_name.clone() },
                            title: title.clone(),
                            wm_class: class_name,
                        };
                        let _ = app.emit("context:change", event);
                    }
                }
            }
        });
    }
}
