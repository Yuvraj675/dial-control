use crate::platform::traits::VirtualInput;
use evdev::uinput::{VirtualDevice, VirtualDeviceBuilder};
use evdev::{AttributeSet, BusType, EventType, InputEvent, InputId, Key};
use std::sync::Mutex;
use std::thread::sleep;
use std::time::Duration;

pub struct LinuxInput {
    device: Mutex<Option<VirtualDevice>>,
}

impl LinuxInput {
    pub fn new() -> Self {
        let mut keys = AttributeSet::<Key>::new();
        for code in 1..255 {
            keys.insert(Key::new(code));
        }

        let input_id = InputId::new(BusType::BUS_USB, 0x046d, 0xc52b, 1);
        let dev = VirtualDeviceBuilder::new()
            .and_then(|builder| {
                builder
                    .name("CMF Buds Pro 2 Virtual Keyboard")
                    .input_id(input_id)
                    .with_keys(&keys)?
                    .build()
            })
            .map_err(|e| {
                eprintln!("[!] Warning: Could not create uinput virtual device: {:?}", e);
                e
            })
            .ok();

        if dev.is_some() {
            println!("[+] Linux Virtual Keyboard initialized via uinput (CMF Buds Pro 2 Virtual Keyboard).");
        }

        Self {
            device: Mutex::new(dev),
        }
    }

    pub fn emit_key(&self, key: Key) {
        if let Ok(mut guard) = self.device.lock() {
            if let Some(ref mut dev) = *guard {
                let down = InputEvent::new(EventType::KEY, key.code(), 1);
                let syn1 = InputEvent::new(EventType::SYNCHRONIZATION, 0, 0);
                let up = InputEvent::new(EventType::KEY, key.code(), 0);
                let syn2 = InputEvent::new(EventType::SYNCHRONIZATION, 0, 0);
                let _ = dev.emit(&[down, syn1]);
                sleep(Duration::from_millis(15));
                let _ = dev.emit(&[up, syn2]);
            }
        }
    }

    pub fn emit_brightness_up(&self) {
        self.emit_key(Key::KEY_BRIGHTNESSUP);
    }

    pub fn emit_brightness_down(&self) {
        self.emit_key(Key::KEY_BRIGHTNESSDOWN);
    }

    pub fn emit_play_pause(&self) {
        self.emit_key(Key::KEY_PLAYPAUSE);
    }

    pub fn emit_next_track(&self) {
        self.emit_key(Key::KEY_NEXTSONG);
    }

    pub fn emit_prev_track(&self) {
        self.emit_key(Key::KEY_PREVIOUSSONG);
    }
}

impl VirtualInput for LinuxInput {
    fn emit_hotkey(&self, key_names: &[String]) {
        if let Ok(mut guard) = self.device.lock() {
            if let Some(ref mut dev) = *guard {
                let mut resolved_keys = Vec::new();
                for name in key_names {
                    if let Some(k) = parse_key_name(name) {
                        resolved_keys.push(k);
                    }
                }

                if resolved_keys.is_empty() {
                    return;
                }

                // Press down in order
                let mut downs: Vec<InputEvent> = resolved_keys
                    .iter()
                    .map(|k| InputEvent::new(EventType::KEY, k.code(), 1))
                    .collect();
                downs.push(InputEvent::new(EventType::SYNCHRONIZATION, 0, 0));
                let _ = dev.emit(&downs);

                sleep(Duration::from_millis(20));

                // Release in reverse order
                let mut ups: Vec<InputEvent> = resolved_keys
                    .iter()
                    .rev()
                    .map(|k| InputEvent::new(EventType::KEY, k.code(), 0))
                    .collect();
                ups.push(InputEvent::new(EventType::SYNCHRONIZATION, 0, 0));
                let _ = dev.emit(&ups);
            }
        }
    }
}

fn parse_key_name(name: &str) -> Option<Key> {
    let clean = name.trim().to_uppercase();
    match clean.as_str() {
        "KEY_LEFTCTRL" | "CTRL" | "LCTRL" => Some(Key::KEY_LEFTCTRL),
        "KEY_RIGHTCTRL" | "RCTRL" => Some(Key::KEY_RIGHTCTRL),
        "KEY_LEFTSHIFT" | "SHIFT" | "LSHIFT" => Some(Key::KEY_LEFTSHIFT),
        "KEY_RIGHTSHIFT" | "RSHIFT" => Some(Key::KEY_RIGHTSHIFT),
        "KEY_LEFTALT" | "ALT" | "LALT" => Some(Key::KEY_LEFTALT),
        "KEY_RIGHTALT" | "RALT" => Some(Key::KEY_RIGHTALT),
        "KEY_LEFTMETA" | "SUPER" | "WIN" => Some(Key::KEY_LEFTMETA),
        "KEY_C" | "C" => Some(Key::KEY_C),
        "KEY_V" | "V" => Some(Key::KEY_V),
        "KEY_X" | "X" => Some(Key::KEY_X),
        "KEY_Z" | "Z" => Some(Key::KEY_Z),
        "KEY_Y" | "Y" => Some(Key::KEY_Y),
        "KEY_A" | "A" => Some(Key::KEY_A),
        "KEY_S" | "S" => Some(Key::KEY_S),
        "KEY_F" | "F" => Some(Key::KEY_F),
        "KEY_M" | "M" => Some(Key::KEY_M),
        "KEY_N" | "N" => Some(Key::KEY_N),
        "KEY_T" | "T" => Some(Key::KEY_T),
        "KEY_W" | "W" => Some(Key::KEY_W),
        "KEY_Q" | "Q" => Some(Key::KEY_Q),
        "KEY_R" | "R" => Some(Key::KEY_R),
        "KEY_DELETE" | "DEL" => Some(Key::KEY_DELETE),
        "KEY_BACKSPACE" => Some(Key::KEY_BACKSPACE),
        "KEY_ENTER" | "RETURN" => Some(Key::KEY_ENTER),
        "KEY_ESC" | "ESCAPE" => Some(Key::KEY_ESC),
        "KEY_SPACE" => Some(Key::KEY_SPACE),
        "KEY_TAB" => Some(Key::KEY_TAB),
        "KEY_LEFT" => Some(Key::KEY_LEFT),
        "KEY_RIGHT" => Some(Key::KEY_RIGHT),
        "KEY_UP" => Some(Key::KEY_UP),
        "KEY_DOWN" => Some(Key::KEY_DOWN),
        "KEY_F2" => Some(Key::KEY_F2),
        "KEY_F5" => Some(Key::KEY_F5),
        "KEY_F6" => Some(Key::KEY_F6),
        "KEY_GRAVE" => Some(Key::KEY_GRAVE),
        "KEY_SLASH" => Some(Key::KEY_SLASH),
        "KEY_BRIGHTNESSUP" => Some(Key::KEY_BRIGHTNESSUP),
        "KEY_BRIGHTNESSDOWN" => Some(Key::KEY_BRIGHTNESSDOWN),
        "KEY_PLAYPAUSE" => Some(Key::KEY_PLAYPAUSE),
        "KEY_NEXTSONG" => Some(Key::KEY_NEXTSONG),
        "KEY_PREVIOUSSONG" => Some(Key::KEY_PREVIOUSSONG),
        _ => None,
    }
}
