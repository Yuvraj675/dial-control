use crate::platform::traits::VirtualInput;
use crate::platform::windows::ffi::*;
use std::mem::size_of;
use std::thread;
use std::time::Duration;

pub struct WindowsInput;

impl WindowsInput {
    pub fn new() -> Self {
        Self
    }
}

fn parse_vk(key_name: &str) -> Option<u16> {
    let clean = key_name.trim().to_lowercase();
    match clean.as_str() {
        "ctrl" | "control" => Some(VK_CONTROL),
        "shift" => Some(VK_SHIFT),
        "alt" | "menu" => Some(VK_MENU),
        "super" | "win" | "meta" | "command" => Some(VK_LWIN),
        "enter" | "return" => Some(VK_RETURN),
        "space" | "spacebar" => Some(VK_SPACE),
        "tab" => Some(VK_TAB),
        "backspace" => Some(VK_BACK),
        "delete" | "del" => Some(VK_DELETE),
        "escape" | "esc" => Some(VK_ESCAPE),
        "left" => Some(VK_LEFT),
        "right" => Some(VK_RIGHT),
        "up" => Some(VK_UP),
        "down" => Some(VK_DOWN),
        "pageup" => Some(VK_PRIOR),
        "pagedown" => Some(VK_NEXT),
        "home" => Some(VK_HOME),
        "end" => Some(VK_END),
        "f1" => Some(0x70),
        "f2" => Some(0x71),
        "f3" => Some(0x72),
        "f4" => Some(0x73),
        "f5" => Some(0x74),
        "f6" => Some(0x75),
        "f7" => Some(0x76),
        "f8" => Some(0x77),
        "f9" => Some(0x78),
        "f10" => Some(0x79),
        "f11" => Some(0x7A),
        "f12" => Some(0x7B),
        s if s.len() == 1 => {
            let c = s.chars().next().unwrap();
            if c.is_ascii_alphabetic() {
                Some(c.to_ascii_uppercase() as u16)
            } else if c.is_ascii_digit() {
                Some(c as u16)
            } else {
                match c {
                    '`' | '~' => Some(0xC0), // VK_OEM_3
                    '-' | '_' => Some(0xBD), // VK_OEM_MINUS
                    '=' | '+' => Some(0xBB), // VK_OEM_PLUS
                    '[' | '{' => Some(0xDB), // VK_OEM_4
                    ']' | '}' => Some(0xDD), // VK_OEM_6
                    '\\' | '|' => Some(0xDC), // VK_OEM_5
                    ';' | ':' => Some(0xBA), // VK_OEM_1
                    '\'' | '"' => Some(0xDE), // VK_OEM_7
                    ',' | '<' => Some(0xBC), // VK_OEM_COMMA
                    '.' | '>' => Some(0xBE), // VK_OEM_PERIOD
                    '/' | '?' => Some(0xBF), // VK_OEM_2
                    _ => None,
                }
            }
        }
        _ => None,
    }
}

fn make_key_input(vk: u16, key_up: bool) -> INPUT {
    let mut flags = 0;
    if key_up {
        flags |= KEYEVENTF_KEYUP;
    }
    // Extended keys: arrow keys, home, end, prior, next, delete, win
    if matches!(
        vk,
        VK_LWIN | VK_RWIN | VK_LEFT | VK_RIGHT | VK_UP | VK_DOWN | VK_DELETE | VK_HOME | VK_END | VK_PRIOR | VK_NEXT
    ) {
        flags |= KEYEVENTF_EXTENDEDKEY;
    }

    let ki = KEYBDINPUT {
        wVk: vk,
        wScan: 0,
        dwFlags: flags,
        time: 0,
        dwExtraInfo: 0,
    };

    INPUT {
        type_: INPUT_KEYBOARD,
        u: INPUT_UNION { ki },
    }
}

impl VirtualInput for WindowsInput {
    fn emit_hotkey(&self, keys: &[String]) {
        if keys.is_empty() {
            return;
        }

        let mut vks: Vec<u16> = Vec::new();
        for key in keys {
            if let Some(vk) = parse_vk(key) {
                vks.push(vk);
            } else {
                eprintln!("[Windows Input] Unknown key code: '{}'", key);
            }
        }

        if vks.is_empty() {
            return;
        }

        println!("[Windows Input] Emitting hotkey: {:?} -> VKs: {:?}", keys, vks);

        unsafe {
            // 1. Press all keys down in order
            let mut down_inputs: Vec<INPUT> = vks.iter().map(|&vk| make_key_input(vk, false)).collect();
            SendInput(
                down_inputs.len() as u32,
                down_inputs.as_mut_ptr(),
                size_of::<INPUT>() as i32,
            );

            // Small delay to allow Windows message loop to register key down
            thread::sleep(Duration::from_millis(15));

            // 2. Release all keys in reverse order
            let mut up_inputs: Vec<INPUT> = vks.iter().rev().map(|&vk| make_key_input(vk, true)).collect();
            SendInput(
                up_inputs.len() as u32,
                up_inputs.as_mut_ptr(),
                size_of::<INPUT>() as i32,
            );
        }
    }
}
