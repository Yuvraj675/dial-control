use crate::platform::traits::{DialBackend, PlatformProvider};
use futures_lite::stream::StreamExt;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager};
use zbus::zvariant;

const KEY_PLAYPAUSE: u16 = 164;
const KEY_PLAYCD: u16 = 200;
const KEY_PAUSECD: u16 = 201;
const KEY_VOICECOMMAND: u16 = 582;

pub struct LinuxDial;

impl LinuxDial {
    pub fn new() -> Self {
        Self
    }
}

pub fn find_cmf_input_device() -> Option<PathBuf> {
    if let Ok(contents) = fs::read_to_string("/proc/bus/input/devices") {
        let entries: Vec<&str> = contents.split("\n\n").collect();
        for entry in &entries {
            if entry.contains("CMF Buds Pro 2") && entry.contains("AVRCP") {
                for line in entry.lines() {
                    if line.starts_with("H: Handlers=") {
                        for token in line.split_whitespace() {
                            if token.starts_with("event") {
                                return Some(PathBuf::from(format!("/dev/input/{}", token)));
                            }
                        }
                    }
                }
            }
        }

        // Fallback: any AVRCP device
        for entry in entries {
            if entry.contains("AVRCP") {
                for line in entry.lines() {
                    if line.starts_with("H: Handlers=") {
                        for token in line.split_whitespace() {
                            if token.starts_with("event") {
                                return Some(PathBuf::from(format!("/dev/input/{}", token)));
                            }
                        }
                    }
                }
            }
        }
    }
    None
}

fn start_input_grabber(
    app: AppHandle,
    is_hud_open: Arc<AtomicBool>,
    _locked_vol: Arc<AtomicI32>,
    is_scroll_mode: Arc<AtomicBool>,
) {
    thread::spawn(move || {
        let mut last_click_time = Instant::now() - Duration::from_secs(1);

        loop {
            let dev_path = match find_cmf_input_device() {
                Some(p) => p,
                None => {
                    thread::sleep(Duration::from_secs(2));
                    continue;
                }
            };

            println!("[+] Attaching Linux input grabber to {:?}", dev_path);
            let mut device = match evdev::Device::open(&dev_path) {
                Ok(d) => d,
                Err(e) => {
                    eprintln!("[!] Could not open input device {:?}: {:?}", dev_path, e);
                    thread::sleep(Duration::from_secs(2));
                    continue;
                }
            };

            if let Err(e) = device.grab() {
                eprintln!("[!] Warning: Could not grab device exclusively: {:?}", e);
            } else {
                println!("[+] Device {:?} grabbed exclusively (OS play/pause blocked).", dev_path);
            }

            loop {
                match device.fetch_events() {
                    Ok(events) => {
                        for ev in events {
                            if ev.event_type() == evdev::EventType::KEY && ev.value() == 1 {
                                let code = ev.code();
                                if code == KEY_PLAYCD
                                    || code == KEY_PAUSECD
                                    || code == KEY_PLAYPAUSE
                                    || code == KEY_VOICECOMMAND
                                {
                                    let now = Instant::now();
                                    if now.duration_since(last_click_time) > Duration::from_millis(500) {
                                        last_click_time = now;

                                        let state: tauri::State<crate::AppState> = app.state();
                                        if is_scroll_mode.load(Ordering::SeqCst) {
                                            println!(">> [NATIVE DIAL CLICK] -> Exiting Scroll Mode");
                                            crate::do_stop_scroll_mode(&app, &state);
                                            continue;
                                        }

                                        let was_open = is_hud_open.load(Ordering::SeqCst);

                                        if !was_open {
                                            println!(">> [NATIVE DIAL CLICK] -> Opening HUD");
                                            crate::do_show_hud(&app, &state);
                                        } else {
                                            println!(">> [NATIVE DIAL CLICK] -> Dispatched action select to HUD");
                                            if let Some(window) = app.get_webview_window("hud") {
                                                let _ = window.emit("dial:click", ());
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("[!] Input fetch error: {:?}. Reconnecting...", e);
                        break;
                    }
                }
            }

            thread::sleep(Duration::from_millis(500));
        }
    });
}

fn start_bluez_listener(
    app: AppHandle,
    is_hud_open: Arc<AtomicBool>,
    locked_vol: Arc<AtomicI32>,
    is_scroll_mode: Arc<AtomicBool>,
) {
    thread::spawn(move || {
        let rt = match tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
        {
            Ok(r) => r,
            Err(e) => {
                eprintln!("[!] Failed to create tokio runtime for BlueZ listener: {:?}", e);
                return;
            }
        };

        rt.block_on(async move {
            let connection = match zbus::Connection::system().await {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("[!] Could not connect to system D-Bus: {:?}", e);
                    return;
                }
            };

            println!("[+] Connected to System D-Bus for BlueZ AVRCP volume tracking.");

            let rule = match zbus::MatchRule::builder()
                .msg_type(zbus::message::Type::Signal)
                .sender("org.bluez")
            {
                Ok(b) => match b.interface("org.freedesktop.DBus.Properties") {
                    Ok(b) => match b.member("PropertiesChanged") {
                        Ok(b) => b.build(),
                        Err(e) => {
                            eprintln!("[!] Rule member failed: {:?}", e);
                            return;
                        }
                    },
                    Err(e) => {
                        eprintln!("[!] Rule interface failed: {:?}", e);
                        return;
                    }
                },
                Err(e) => {
                    eprintln!("[!] Rule sender failed: {:?}", e);
                    return;
                }
            };

            let mut stream = match zbus::MessageStream::for_match_rule(rule, &connection, None).await {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("[!] Failed to create MessageStream: {:?}", e);
                    return;
                }
            };

            let mut last_raw: Option<i32> = None;
            let mut locked_raw_baseline: Option<i32> = None;
            let mut restore_pending = false;
            let mut last_restore_time = Instant::now() - Duration::from_secs(10);
            let mut was_locked = false;

            while let Some(msg) = stream.next().await {
                if let Ok(msg) = msg {
                    let path = msg.header().path().map(|p| p.to_string()).unwrap_or_default();
                    if !path.contains("dev_") && !path.contains("sep") {
                        continue;
                    }

                    if let Ok((iface, changed, _)) =
                        msg.body().deserialize::<(String, HashMap<String, zvariant::OwnedValue>, Vec<String>)>()
                    {
                        if iface != "org.bluez.MediaTransport1" {
                            continue;
                        }

                        if let Some(vol_val) = changed.get("Volume") {
                            let new_vol: i32 = match vol_val.try_into() {
                                Ok(v) => {
                                    let u: u16 = v;
                                    u as i32
                                }
                                Err(_) => continue,
                            };

                            let is_hud = is_hud_open.load(Ordering::SeqCst);
                            let is_scroll = is_scroll_mode.load(Ordering::SeqCst);
                            let locked_pct = locked_vol.load(Ordering::SeqCst);

                            // Locking is active when HUD is open or in scroll mode, EXCEPT when in volume mode (locked_pct < 0)
                            let lock_active = (is_hud || is_scroll) && (locked_pct >= 0);

                            // Detect entering or exiting locked mode
                            if lock_active && !was_locked {
                                was_locked = true;
                                restore_pending = false;
                                let raw = match last_raw {
                                    Some(v) => v,
                                    None => ((locked_pct as f64 * 127.0) / 100.0).round() as i32,
                                };
                                let clamped_baseline = raw.clamp(8, 119);
                                locked_raw_baseline = Some(clamped_baseline);
                                last_raw = Some(clamped_baseline);

                                println!(
                                    "[BlueZ Dial] Volume lock ACTIVATED at baseline raw: {} (mapped: {}%)",
                                    clamped_baseline,
                                    ((clamped_baseline as f64 / 127.0) * 100.0).round() as i32
                                );

                                if raw != clamped_baseline {
                                    let proxy = zbus::Proxy::new(
                                        &connection,
                                        "org.bluez",
                                        path.clone(),
                                        "org.freedesktop.DBus.Properties",
                                    )
                                    .await;
                                    if let Ok(proxy) = proxy {
                                        let val = zvariant::Value::U16(clamped_baseline as u16);
                                        let _ = proxy
                                            .call_method("Set", &("org.bluez.MediaTransport1", "Volume", val))
                                            .await;
                                    }
                                }
                            } else if !lock_active && was_locked {
                                was_locked = false;
                                restore_pending = false;
                                locked_raw_baseline = None;
                                println!("[BlueZ Dial] Volume lock RELEASED (natural hardware volume control)");
                            }

                            if lock_active {
                                let baseline = locked_raw_baseline.unwrap_or(64);

                                // Safety timeout for pending restore
                                if restore_pending && last_restore_time.elapsed() > Duration::from_millis(400) {
                                    restore_pending = false;
                                }

                                let prev = last_raw.unwrap_or(baseline);
                                let dist_to_base = (new_vol - baseline).abs();
                                let prev_dist_to_base = (prev - baseline).abs();

                                // Echo suppression: If new_vol is returning towards or reached baseline, swallow it!
                                if dist_to_base <= 3 || (restore_pending && dist_to_base < prev_dist_to_base) {
                                    restore_pending = false;
                                    last_raw = Some(baseline);
                                    continue;
                                }

                                let delta = new_vol - prev;
                                if delta == 0 {
                                    continue;
                                }

                                let direction = if delta > 0 { 1 } else { -1 };
                                let dir_name = if direction > 0 { "CW (+)" } else { "CCW (-)" };

                                println!(
                                    ">> [NATIVE DIAL TURN] {} (Raw: {}, Baseline: {}, Delta: {})",
                                    dir_name, new_vol, baseline, delta
                                );

                                last_raw = Some(new_vol);
                                restore_pending = true;
                                last_restore_time = Instant::now();

                                if is_scroll {
                                    // direction > 0 is clockwise -> scroll down (-1)
                                    // direction < 0 is counter-clockwise -> scroll up (1)
                                    let scroll_delta = if direction > 0 { -1 } else { 1 };
                                    let app_clone = app.clone();
                                    thread::spawn(move || {
                                        let state: tauri::State<crate::AppState> = app_clone.state();
                                        state.platform.input().emit_scroll(scroll_delta);
                                        let _ = app_clone.emit("scroll:tick", serde_json::json!({ "direction": direction }));
                                    });
                                } else {
                                    if let Some(window) = app.get_webview_window("hud") {
                                        let mapped_pct = ((baseline as f64 / 127.0) * 100.0).round().clamp(0.0, 100.0) as i32;
                                        let _ = window.emit(
                                            "dial:rotate",
                                            serde_json::json!({
                                                "direction": direction,
                                                "delta": direction,
                                                "volume": baseline,
                                                "volume_pct": mapped_pct
                                            }),
                                        );
                                    }
                                }

                                // Reset BlueZ volume back to baseline directly over D-Bus
                                let proxy = zbus::Proxy::new(
                                    &connection,
                                    "org.bluez",
                                    path.clone(),
                                    "org.freedesktop.DBus.Properties",
                                )
                                .await;

                                if let Ok(proxy) = proxy {
                                    let val = zvariant::Value::U16(baseline as u16);
                                    let _ = proxy
                                        .call_method("Set", &("org.bluez.MediaTransport1", "Volume", val))
                                        .await;
                                }

                                continue;
                            }

                            // Natural hardware volume control (Volume adjustment mode or HUD closed)
                            let delta = match last_raw {
                                Some(prev) => new_vol - prev,
                                None => {
                                    last_raw = Some(new_vol);
                                    continue;
                                }
                            };
                            last_raw = Some(new_vol);

                            if delta == 0 {
                                continue;
                            }

                            let direction = if delta > 0 { 1 } else { -1 };
                            let dir_name = if delta > 0 { "CW (+)" } else { "CCW (-)" };
                            let mapped_pct = ((new_vol as f64 / 127.0) * 100.0)
                                .round()
                                .clamp(0.0, 100.0) as i32;

                            println!(
                                ">> [NATIVE VOLUME CHANGE] {} (Raw AVRCP: {}, Mapped: {}%)",
                                dir_name, new_vol, mapped_pct
                            );

                            if let Some(window) = app.get_webview_window("hud") {
                                let _ = window.emit(
                                    "dial:rotate",
                                    serde_json::json!({
                                        "direction": direction,
                                        "delta": delta,
                                        "volume": new_vol,
                                        "volume_pct": mapped_pct
                                    }),
                                );
                            }
                        }
                    }
                }
            }
        });
    });
}

impl DialBackend for LinuxDial {
    fn start_listeners(
        &self,
        app: AppHandle,
        is_hud_open: Arc<AtomicBool>,
        locked_vol: Arc<AtomicI32>,
        is_scroll_mode: Arc<AtomicBool>,
    ) {
        start_input_grabber(app.clone(), is_hud_open.clone(), locked_vol.clone(), is_scroll_mode.clone());
        start_bluez_listener(app, is_hud_open, locked_vol, is_scroll_mode);
    }
}
