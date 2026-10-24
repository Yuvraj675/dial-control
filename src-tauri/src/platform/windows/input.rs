use crate::platform::keymap::windows_vk;
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
        dwExtraInfo: SYNTHETIC_INPUT_TAG,
    };

    INPUT {
        type_: INPUT_KEYBOARD,
        u: INPUT_UNION { ki },
    }
}

impl VirtualInput for WindowsInput {
    fn emit_scroll(&self, delta: i32) {
        let amount = (delta * 120) as i32 as DWORD;
        let mi = MOUSEINPUT {
            dx: 0,
            dy: 0,
            mouseData: amount,
            dwFlags: MOUSEEVENTF_WHEEL,
            time: 0,
            dwExtraInfo: SYNTHETIC_INPUT_TAG,
        };
        let input = INPUT {
            type_: INPUT_MOUSE,
            u: INPUT_UNION { mi },
        };
        unsafe {
            SendInput(1, &input, size_of::<INPUT>() as i32);
        }
    }

    fn emit_hotkey(&self, keys: &[String]) {
        if keys.is_empty() {
            return;
        }

        let mut vks: Vec<u16> = Vec::new();
        for key in keys {
            if let Some(vk) = windows_vk(key) {
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
