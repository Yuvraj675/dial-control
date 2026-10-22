use crate::platform::traits::VirtualInput;
use crate::platform::windows::context::get_last_target_hwnd;
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
    let raw = key_name.trim().to_lowercase();
    let clean = raw.strip_prefix("key_").unwrap_or(&raw);

    match clean {
        "ctrl" | "control" | "leftctrl" | "rightctrl" => Some(VK_CONTROL),
        "shift" | "leftshift" | "rightshift" => Some(VK_SHIFT),
        "alt" | "menu" | "leftalt" | "rightalt" => Some(VK_MENU),
        "super" | "win" | "meta" | "command" | "leftmeta" | "rightmeta" => Some(VK_LWIN),
        "enter" | "return" => Some(VK_RETURN),
        "space" | "spacebar" => Some(VK_SPACE),
        "tab" => Some(VK_TAB),
        "backspace" | "back" => Some(VK_BACK),
        "delete" | "del" => Some(VK_DELETE),
        "escape" | "esc" => Some(VK_ESCAPE),
        "left" => Some(VK_LEFT),
        "right" => Some(VK_RIGHT),
        "up" => Some(VK_UP),
        "down" => Some(VK_DOWN),
        "pageup" | "page_up" => Some(VK_PRIOR),
        "pagedown" | "page_down" => Some(VK_NEXT),
        "home" => Some(VK_HOME),
        "end" => Some(VK_END),
        "insert" => Some(VK_INSERT),
        "slash" => Some(0xBF), // VK_OEM_2
        "backslash" => Some(0xDC), // VK_OEM_5
        "grave" | "backtick" => Some(0xC0), // VK_OEM_3
        "minus" => Some(0xBD), // VK_OEM_MINUS
        "equal" | "plus" => Some(0xBB), // VK_OEM_PLUS
        "leftbrace" | "bracketleft" => Some(0xDB), // VK_OEM_4
        "rightbrace" | "bracketright" => Some(0xDD), // VK_OEM_6
        "semicolon" => Some(0xBA), // VK_OEM_1
        "apostrophe" | "quote" => Some(0xDE), // VK_OEM_7
        "comma" => Some(0xBC), // VK_OEM_COMMA
        "dot" | "period" => Some(0xBE), // VK_OEM_PERIOD
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
            // Restore foreground focus to previous target window if known
            let target_hwnd = get_last_target_hwnd();
            if !target_hwnd.is_null() {
                SetForegroundWindow(target_hwnd);
                thread::sleep(Duration::from_millis(60));
            }

            // 1. Press all keys down in order
            let mut down_inputs: Vec<INPUT> = vks.iter().map(|&vk| make_key_input(vk, false)).collect();
            SendInput(
                down_inputs.len() as u32,
                down_inputs.as_mut_ptr(),
                size_of::<INPUT>() as i32,
            );

            // Small delay to allow target window to register key down
            thread::sleep(Duration::from_millis(25));

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
