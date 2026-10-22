use crate::platform::traits::SystemAudio;
use crate::platform::windows::ffi::*;
use std::mem::size_of;
use std::sync::atomic::{AtomicI32, Ordering};
use std::thread;
use std::time::Duration;

pub struct WindowsAudio {
    cached_vol: AtomicI32,
}

impl WindowsAudio {
    pub fn new() -> Self {
        Self {
            cached_vol: AtomicI32::new(50),
        }
    }

    fn send_audio_key(&self, vk: u16) {
        unsafe {
            let ki_down = KEYBDINPUT {
                wVk: vk,
                wScan: 0,
                dwFlags: KEYEVENTF_EXTENDEDKEY,
                time: 0,
                dwExtraInfo: 0,
            };
            let mut down = INPUT {
                type_: INPUT_KEYBOARD,
                u: INPUT_UNION { ki: ki_down },
            };
            SendInput(1, &mut down, size_of::<INPUT>() as i32);

            thread::sleep(Duration::from_millis(5));

            let ki_up = KEYBDINPUT {
                wVk: vk,
                wScan: 0,
                dwFlags: KEYEVENTF_EXTENDEDKEY | KEYEVENTF_KEYUP,
                time: 0,
                dwExtraInfo: 0,
            };
            let mut up = INPUT {
                type_: INPUT_KEYBOARD,
                u: INPUT_UNION { ki: ki_up },
            };
            SendInput(1, &mut up, size_of::<INPUT>() as i32);
        }
    }
}

impl SystemAudio for WindowsAudio {
    fn get_volume(&self) -> i32 {
        self.cached_vol.load(Ordering::SeqCst)
    }

    fn set_volume(&self, pct: i32) -> i32 {
        let target = pct.clamp(0, 100);
        let current = self.cached_vol.load(Ordering::SeqCst);
        let diff = target - current;

        if diff > 0 {
            let steps = (diff + 1) / 2;
            for _ in 0..steps {
                self.send_audio_key(VK_VOLUME_UP);
            }
        } else if diff < 0 {
            let steps = (-diff + 1) / 2;
            for _ in 0..steps {
                self.send_audio_key(VK_VOLUME_DOWN);
            }
        }

        self.cached_vol.store(target, Ordering::SeqCst);
        println!("[Windows Audio] Master volume set to {}%", target);
        target
    }

    fn toggle_mute(&self) {
        println!("[Windows Audio] Mute toggled");
        self.send_audio_key(VK_VOLUME_MUTE);
    }
}
