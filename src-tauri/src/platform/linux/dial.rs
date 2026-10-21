use crate::platform::traits::DialBackend;
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

const CMF_MAC_FORMATTED: &str = "2C_BE_EE_31_11_77";

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
    locked_vol: Arc<AtomicI32>,
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
                                        let was_open = is_hud_open.load(Ordering::SeqCst);

                                        if !was_open {
                                            println!(">> [NATIVE DIAL CLICK] -> Opening HUD on Wayland");
                                            is_hud_open.store(true, Ordering::SeqCst);
                                            locked_vol.store(50, Ordering::SeqCst);
                                            if let Some(window) = app.get_webview_window("hud") {
                                                let _ = window.show();
                                                let _ = window.unminimize();
                                                let _ = window.set_focus();
                                                let _ = window.set_always_on_top(true);
                                                let _ = window.emit("hud:visibility", true);
                                            }
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

            let mut last_vol: Option<i32> = None;
            let mut ignore_restore_vol: Option<i32> = None;

            while let Some(msg) = stream.next().await {
                if let Ok(msg) = msg {
                    let path = msg.header().path().map(|p| p.to_string()).unwrap_or_default();
                    if !path.contains(CMF_MAC_FORMATTED) && !path.contains("sep") {
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

                            if let Some(ignore_v) = ignore_restore_vol {
                                if (new_vol - ignore_v).abs() <= 2 {
                                    ignore_restore_vol = None;
                                    last_vol = Some(new_vol);
                                    continue;
                                }
                            }

                            let delta = match last_vol {
                                Some(prev) => new_vol - prev,
                                None => {
                                    last_vol = Some(new_vol);
                                    continue;
                                }
                            };
                            last_vol = Some(new_vol);

                            if delta == 0 {
                                continue;
                            }

                            let direction = if delta > 0 { 1 } else { -1 };
                            let dir_name = if delta > 0 { "CW (+)" } else { "CCW (-)" };
                            let mapped_pct = ((new_vol as f64 / 127.0) * 100.0)
                                .round()
                                .clamp(0.0, 100.0) as i32;
                            println!(
                                ">> [NATIVE DIAL TURN] {} (Raw AVRCP: {}, Mapped: {}%)",
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

                            let hud_open = is_hud_open.load(Ordering::SeqCst);
                            let locked_v = locked_vol.load(Ordering::SeqCst);

                            if hud_open && locked_v > 0 {
                                ignore_restore_vol = Some(locked_v);
                                let proxy = zbus::Proxy::new(
                                    &connection,
                                    "org.bluez",
                                    path.clone(),
                                    "org.freedesktop.DBus.Properties",
                                )
                                .await;

                                if let Ok(proxy) = proxy {
                                    let val: zvariant::Value = zvariant::Value::U16(locked_v as u16);
                                    let _ = proxy
                                        .call_method(
                                            "Set",
                                            &("org.bluez.MediaTransport1", "Volume", val),
                                        )
                                        .await;
                                }
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
    ) {
        start_input_grabber(app.clone(), is_hud_open.clone(), locked_vol.clone());
        start_bluez_listener(app, is_hud_open, locked_vol);
    }
}
