use crate::platform::traits::MediaControl;
use crate::platform::windows::ffi::*;
use std::mem::size_of;
use std::thread;
use std::time::Duration;

pub struct WindowsMedia;

impl WindowsMedia {
    pub fn new() -> Self {
        Self
    }

    fn send_media_key(&self, vk: u16) {
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

            thread::sleep(Duration::from_millis(15));

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

impl MediaControl for WindowsMedia {
    fn play_pause(&self) {
        println!("[Windows Media] Play/Pause toggled");
        self.send_media_key(VK_MEDIA_PLAY_PAUSE);
    }

    fn next_track(&self) {
        println!("[Windows Media] Next track");
        self.send_media_key(VK_MEDIA_NEXT_TRACK);
    }

    fn prev_track(&self) {
        println!("[Windows Media] Prev track");
        self.send_media_key(VK_MEDIA_PREV_TRACK);
    }
}
