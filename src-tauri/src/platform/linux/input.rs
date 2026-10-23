use crate::platform::traits::VirtualInput;
use evdev::uinput::{VirtualDevice, VirtualDeviceBuilder};
use evdev::{AttributeSet, BusType, EventType, InputEvent, InputId, Key, RelativeAxisType};
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

        let mut rel_axes = AttributeSet::<RelativeAxisType>::new();
        rel_axes.insert(RelativeAxisType::REL_WHEEL);
        rel_axes.insert(RelativeAxisType::REL_HWHEEL);

        let input_id = InputId::new(BusType::BUS_USB, 0x046d, 0xc52b, 1);
        let dev = VirtualDeviceBuilder::new()
            .and_then(|builder| {
                builder
                    .name("CMF Buds Pro 2 Virtual Device")
                    .input_id(input_id)
                    .with_keys(&keys)?
                    .with_relative_axes(&rel_axes)?
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
                sleep(Duration::from_millis(25));
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

    pub fn emit_scroll(&self, delta: i32) {
        if let Ok(mut guard) = self.device.lock() {
            if let Some(ref mut dev) = *guard {
                let ev = InputEvent::new(EventType::RELATIVE, RelativeAxisType::REL_WHEEL.0, delta);
                let syn = InputEvent::new(EventType::SYNCHRONIZATION, 0, 0);
                let _ = dev.emit(&[ev, syn]);
            }
        }
    }
}

impl VirtualInput for LinuxInput {
    fn emit_scroll(&self, delta: i32) {
        self.emit_scroll(delta);
    }

    fn emit_hotkey(&self, key_names: &[String]) {
        if let Ok(mut guard) = self.device.lock() {
            if let Some(ref mut dev) = *guard {
                let mut resolved_keys = Vec::new();
                for name in key_names {
                    if let Some(k) = parse_key_name(name) {
                        resolved_keys.push(k);
                    } else {
                        eprintln!("[!] Warning: Unknown key name in hotkey: {}", name);
                    }
                }

                if resolved_keys.is_empty() {
                    return;
                }

                println!("[Input] Emitting sequence for keys: {:?}", resolved_keys);

                // 1. Press down keys sequentially with micro-delay and SYN_REPORT
                for k in &resolved_keys {
                    let down = InputEvent::new(EventType::KEY, k.code(), 1);
                    let syn = InputEvent::new(EventType::SYNCHRONIZATION, 0, 0);
                    let _ = dev.emit(&[down, syn]);
                    sleep(Duration::from_millis(15));
                }

                // 2. Hold combination so the window manager and target app process modifier states
                sleep(Duration::from_millis(50));

                // 3. Release keys in reverse order sequentially
                for k in resolved_keys.iter().rev() {
                    let up = InputEvent::new(EventType::KEY, k.code(), 0);
                    let syn = InputEvent::new(EventType::SYNCHRONIZATION, 0, 0);
                    let _ = dev.emit(&[up, syn]);
                    sleep(Duration::from_millis(15));
                }
            }
        }
    }
}

fn parse_key_name(name: &str) -> Option<Key> {
    let clean = name.trim().to_uppercase();
    match clean.as_str() {
        // Modifiers
        "KEY_LEFTCTRL" | "CTRL" | "LCTRL" => Some(Key::KEY_LEFTCTRL),
        "KEY_RIGHTCTRL" | "RCTRL" => Some(Key::KEY_RIGHTCTRL),
        "KEY_LEFTSHIFT" | "SHIFT" | "LSHIFT" => Some(Key::KEY_LEFTSHIFT),
        "KEY_RIGHTSHIFT" | "RSHIFT" => Some(Key::KEY_RIGHTSHIFT),
        "KEY_LEFTALT" | "ALT" | "LALT" => Some(Key::KEY_LEFTALT),
        "KEY_RIGHTALT" | "RALT" => Some(Key::KEY_RIGHTALT),
        "KEY_LEFTMETA" | "SUPER" | "WIN" | "META" => Some(Key::KEY_LEFTMETA),
        "KEY_RIGHTMETA" => Some(Key::KEY_RIGHTMETA),

        // Letters A-Z
        "KEY_A" | "A" => Some(Key::KEY_A),
        "KEY_B" | "B" => Some(Key::KEY_B),
        "KEY_C" | "C" => Some(Key::KEY_C),
        "KEY_D" | "D" => Some(Key::KEY_D),
        "KEY_E" | "E" => Some(Key::KEY_E),
        "KEY_F" | "F" => Some(Key::KEY_F),
        "KEY_G" | "G" => Some(Key::KEY_G),
        "KEY_H" | "H" => Some(Key::KEY_H),
        "KEY_I" | "I" => Some(Key::KEY_I),
        "KEY_J" | "J" => Some(Key::KEY_J),
        "KEY_K" | "K" => Some(Key::KEY_K),
        "KEY_L" | "L" => Some(Key::KEY_L),
        "KEY_M" | "M" => Some(Key::KEY_M),
        "KEY_N" | "N" => Some(Key::KEY_N),
        "KEY_O" | "O" => Some(Key::KEY_O),
        "KEY_P" | "P" => Some(Key::KEY_P),
        "KEY_Q" | "Q" => Some(Key::KEY_Q),
        "KEY_R" | "R" => Some(Key::KEY_R),
        "KEY_S" | "S" => Some(Key::KEY_S),
        "KEY_T" | "T" => Some(Key::KEY_T),
        "KEY_U" | "U" => Some(Key::KEY_U),
        "KEY_V" | "V" => Some(Key::KEY_V),
        "KEY_W" | "W" => Some(Key::KEY_W),
        "KEY_X" | "X" => Some(Key::KEY_X),
        "KEY_Y" | "Y" => Some(Key::KEY_Y),
        "KEY_Z" | "Z" => Some(Key::KEY_Z),

        // Numbers 0-9
        "KEY_0" | "0" => Some(Key::KEY_0),
        "KEY_1" | "1" => Some(Key::KEY_1),
        "KEY_2" | "2" => Some(Key::KEY_2),
        "KEY_3" | "3" => Some(Key::KEY_3),
        "KEY_4" | "4" => Some(Key::KEY_4),
        "KEY_5" | "5" => Some(Key::KEY_5),
        "KEY_6" | "6" => Some(Key::KEY_6),
        "KEY_7" | "7" => Some(Key::KEY_7),
        "KEY_8" | "8" => Some(Key::KEY_8),
        "KEY_9" | "9" => Some(Key::KEY_9),

        // Function Keys F1-F12
        "KEY_F1" | "F1" => Some(Key::KEY_F1),
        "KEY_F2" | "F2" => Some(Key::KEY_F2),
        "KEY_F3" | "F3" => Some(Key::KEY_F3),
        "KEY_F4" | "F4" => Some(Key::KEY_F4),
        "KEY_F5" | "F5" => Some(Key::KEY_F5),
        "KEY_F6" | "F6" => Some(Key::KEY_F6),
        "KEY_F7" | "F7" => Some(Key::KEY_F7),
        "KEY_F8" | "F8" => Some(Key::KEY_F8),
        "KEY_F9" | "F9" => Some(Key::KEY_F9),
        "KEY_F10" | "F10" => Some(Key::KEY_F10),
        "KEY_F11" | "F11" => Some(Key::KEY_F11),
        "KEY_F12" | "F12" => Some(Key::KEY_F12),

        // Navigation & Control
        "KEY_ESC" | "ESCAPE" | "ESC" => Some(Key::KEY_ESC),
        "KEY_ENTER" | "RETURN" | "ENTER" => Some(Key::KEY_ENTER),
        "KEY_SPACE" | "SPACE" => Some(Key::KEY_SPACE),
        "KEY_TAB" | "TAB" => Some(Key::KEY_TAB),
        "KEY_BACKSPACE" | "BACKSPACE" => Some(Key::KEY_BACKSPACE),
        "KEY_DELETE" | "DELETE" | "DEL" => Some(Key::KEY_DELETE),
        "KEY_LEFT" | "LEFT" => Some(Key::KEY_LEFT),
        "KEY_RIGHT" | "RIGHT" => Some(Key::KEY_RIGHT),
        "KEY_UP" | "UP" => Some(Key::KEY_UP),
        "KEY_DOWN" | "DOWN" => Some(Key::KEY_DOWN),
        "KEY_HOME" => Some(Key::KEY_HOME),
        "KEY_END" => Some(Key::KEY_END),
        "KEY_PAGEUP" | "PAGE_UP" => Some(Key::KEY_PAGEUP),
        "KEY_PAGEDOWN" | "PAGE_DOWN" => Some(Key::KEY_PAGEDOWN),

        // Symbols & Punctuation
        "KEY_SLASH" | "SLASH" | "/" => Some(Key::KEY_SLASH),
        "KEY_GRAVE" | "GRAVE" | "`" => Some(Key::KEY_GRAVE),
        "KEY_MINUS" | "MINUS" | "-" => Some(Key::KEY_MINUS),
        "KEY_EQUAL" | "EQUAL" | "=" => Some(Key::KEY_EQUAL),
        "KEY_COMMA" | "," => Some(Key::KEY_COMMA),
        "KEY_DOT" | "." => Some(Key::KEY_DOT),
        "KEY_SEMICOLON" | ";" => Some(Key::KEY_SEMICOLON),
        "KEY_APOSTROPHE" | "'" => Some(Key::KEY_APOSTROPHE),
        "KEY_LEFTBRACE" | "[" => Some(Key::KEY_LEFTBRACE),
        "KEY_RIGHTBRACE" | "]" => Some(Key::KEY_RIGHTBRACE),
        "KEY_BACKSLASH" | "\\" => Some(Key::KEY_BACKSLASH),

        // Media & Hardware
        "KEY_PLAYPAUSE" => Some(Key::KEY_PLAYPAUSE),
        "KEY_NEXTSONG" => Some(Key::KEY_NEXTSONG),
        "KEY_PREVIOUSSONG" => Some(Key::KEY_PREVIOUSSONG),
        "KEY_BRIGHTNESSUP" => Some(Key::KEY_BRIGHTNESSUP),
        "KEY_BRIGHTNESSDOWN" => Some(Key::KEY_BRIGHTNESSDOWN),

        _ => None,
    }
}
