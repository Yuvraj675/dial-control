use crate::platform::traits::*;
use std::sync::atomic::{AtomicBool, AtomicI32};
use std::sync::Arc;
use tauri::AppHandle;

pub struct WindowsAudio;
impl SystemAudio for WindowsAudio {
    fn get_volume(&self) -> i32 {
        println!("[Windows] Audio: get_volume stub");
        50
    }
    fn set_volume(&self, pct: i32) -> i32 {
        println!("[Windows] Audio: set_volume to {}% stub", pct);
        pct
    }
    fn toggle_mute(&self) {
        println!("[Windows] Audio: toggle_mute stub");
    }
}

pub struct WindowsBrightness;
impl SystemBrightness for WindowsBrightness {
    fn get_brightness(&self) -> i32 {
        println!("[Windows] Brightness: get_brightness stub");
        50
    }
    fn set_brightness(&self, pct: i32) -> i32 {
        println!("[Windows] Brightness: set_brightness to {}% stub", pct);
        pct
    }
    fn step_brightness(&self, direction: i32) {
        println!("[Windows] Brightness: step_brightness direction {} stub", direction);
    }
}

pub struct WindowsMedia;
impl MediaControl for WindowsMedia {
    fn play_pause(&self) {
        println!("[Windows] Media: play_pause stub");
    }
    fn next_track(&self) {
        println!("[Windows] Media: next_track stub");
    }
    fn prev_track(&self) {
        println!("[Windows] Media: prev_track stub");
    }
}

pub struct WindowsInput;
impl VirtualInput for WindowsInput {
    fn emit_hotkey(&self, keys: &[String]) {
        println!("[Windows] Input: emit_hotkey {:?} stub", keys);
    }
}

pub struct WindowsDial;
impl DialBackend for WindowsDial {
    fn start_listeners(
        &self,
        _app: AppHandle,
        _is_hud_open: Arc<AtomicBool>,
        _locked_vol: Arc<AtomicI32>,
    ) {
        println!("[Windows] Dial: start_listeners stub");
    }
}

pub struct WindowsContext;
impl ContextDetector for WindowsContext {
    fn start(&self, _app: AppHandle) {
        println!("[Windows] Context: start stub");
    }
}

pub struct Platform {
    audio: WindowsAudio,
    brightness: WindowsBrightness,
    media: WindowsMedia,
    input: WindowsInput,
    dial: WindowsDial,
    context: WindowsContext,
}

impl Platform {
    pub fn new() -> Self {
        Self {
            audio: WindowsAudio,
            brightness: WindowsBrightness,
            media: WindowsMedia,
            input: WindowsInput,
            dial: WindowsDial,
            context: WindowsContext,
        }
    }
}

impl PlatformProvider for Platform {
    fn audio(&self) -> &dyn SystemAudio {
        &self.audio
    }
    fn brightness(&self) -> &dyn SystemBrightness {
        &self.brightness
    }
    fn media(&self) -> &dyn MediaControl {
        &self.media
    }
    fn input(&self) -> &dyn VirtualInput {
        &self.input
    }
    fn dial(&self) -> &dyn DialBackend {
        &self.dial
    }
    fn context(&self) -> &dyn ContextDetector {
        &self.context
    }
}
