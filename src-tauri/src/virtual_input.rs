use evdev::uinput::{VirtualDevice, VirtualDeviceBuilder};
use evdev::{AttributeSet, Key, InputEvent, EventType};
use std::sync::Mutex;
use std::thread::sleep;
use std::time::Duration;

pub struct VirtualKeyboard {
    device: Mutex<Option<VirtualDevice>>,
}

impl VirtualKeyboard {
    pub fn new() -> Self {
        let mut keys = AttributeSet::<Key>::new();
        // Support keys from 1 to 255 for media, navigation, and function keys
        for code in 1..255 {
            keys.insert(Key::new(code));
        }

        let dev = VirtualDeviceBuilder::new()
            .and_then(|builder| builder.name("CMF Dial Virtual Keyboard").with_keys(&keys)?.build())
            .map_err(|e| {
                eprintln!("[!] Warning: Could not create uinput virtual device: {:?}", e);
                e
            })
            .ok();

        if dev.is_some() {
            println!("[+] Virtual Keyboard initialized via Linux uinput.");
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

    pub fn emit_key_combination(&self, key_names: &[String]) {
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
    match name.to_uppercase().as_str() {
        "KEY_LEFTCTRL" | "CTRL" => Some(Key::KEY_LEFTCTRL),
        "KEY_RIGHTCTRL" => Some(Key::KEY_RIGHTCTRL),
        "KEY_LEFTSHIFT" | "SHIFT" => Some(Key::KEY_LEFTSHIFT),
        "KEY_RIGHTSHIFT" => Some(Key::KEY_RIGHTSHIFT),
        "KEY_LEFTALT" | "ALT" => Some(Key::KEY_LEFTALT),
        "KEY_RIGHTALT" => Some(Key::KEY_RIGHTALT),
        "KEY_LEFTMETA" | "SUPER" | "META" => Some(Key::KEY_LEFTMETA),
        "KEY_C" => Some(Key::KEY_C),
        "KEY_V" => Some(Key::KEY_V),
        "KEY_X" => Some(Key::KEY_X),
        "KEY_Z" => Some(Key::KEY_Z),
        "KEY_Y" => Some(Key::KEY_Y),
        "KEY_S" => Some(Key::KEY_S),
        "KEY_T" => Some(Key::KEY_T),
        "KEY_W" => Some(Key::KEY_W),
        "KEY_R" => Some(Key::KEY_R),
        "KEY_F" => Some(Key::KEY_F),
        "KEY_H" => Some(Key::KEY_H),
        "KEY_N" => Some(Key::KEY_N),
        "KEY_P" => Some(Key::KEY_P),
        "KEY_F2" => Some(Key::KEY_F2),
        "KEY_F5" => Some(Key::KEY_F5),
        "KEY_F6" => Some(Key::KEY_F6),
        "KEY_DELETE" | "DEL" => Some(Key::KEY_DELETE),
        "KEY_EQUAL" | "=" => Some(Key::KEY_EQUAL),
        "KEY_MINUS" | "-" => Some(Key::KEY_MINUS),
        "KEY_LEFT" => Some(Key::KEY_LEFT),
        "KEY_RIGHT" => Some(Key::KEY_RIGHT),
        "KEY_UP" => Some(Key::KEY_UP),
        "KEY_DOWN" => Some(Key::KEY_DOWN),
        "KEY_PLAYPAUSE" => Some(Key::KEY_PLAYPAUSE),
        "KEY_NEXTSONG" => Some(Key::KEY_NEXTSONG),
        "KEY_PREVIOUSSONG" => Some(Key::KEY_PREVIOUSSONG),
        "KEY_MUTE" => Some(Key::KEY_MUTE),
        "KEY_VOLUMEUP" => Some(Key::KEY_VOLUMEUP),
        "KEY_VOLUMEDOWN" => Some(Key::KEY_VOLUMEDOWN),
        _ => None,
    }
}
