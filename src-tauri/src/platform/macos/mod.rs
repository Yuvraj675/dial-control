use crate::platform::traits::*;
use std::sync::atomic::{AtomicBool, AtomicI32};
use std::sync::Arc;
use tauri::AppHandle;

pub struct MacAudio;
impl SystemAudio for MacAudio {
    fn get_volume(&self) -> i32 {
        println!("[macOS] Audio: get_volume stub");
        50
    }
    fn set_volume(&self, pct: i32) -> i32 {
        println!("[macOS] Audio: set_volume to {}% stub", pct);
        pct
    }
    fn toggle_mute(&self) {
        println!("[macOS] Audio: toggle_mute stub");
    }
}

pub struct MacBrightness;
impl SystemBrightness for MacBrightness {
    fn get_brightness(&self) -> i32 {
        println!("[macOS] Brightness: get_brightness stub");
        50
    }
    fn set_brightness(&self, pct: i32) -> i32 {
        println!("[macOS] Brightness: set_brightness to {}% stub", pct);
        pct
    }
    fn step_brightness(&self, direction: i32) {
        println!("[macOS] Brightness: step_brightness direction {} stub", direction);
    }
}

pub struct MacMedia;
impl MediaControl for MacMedia {
    fn play_pause(&self) {
        println!("[macOS] Media: play_pause stub");
    }
    fn next_track(&self) {
        println!("[macOS] Media: next_track stub");
    }
    fn prev_track(&self) {
        println!("[macOS] Media: prev_track stub");
    }
}

pub struct MacInput;
impl VirtualInput for MacInput {
    fn emit_hotkey(&self, keys: &[String]) {
        println!("[macOS] Input: emit_hotkey {:?} stub", keys);
    }
}

pub struct MacDial;
impl DialBackend for MacDial {
    fn start_listeners(
        &self,
        _app: AppHandle,
        _is_hud_open: Arc<AtomicBool>,
        _locked_vol: Arc<AtomicI32>,
    ) {
        println!("[macOS] Dial: start_listeners stub");
    }
}

pub struct MacContext;
impl ContextDetector for MacContext {
    fn start(&self, _app: AppHandle) {
        println!("[macOS] Context: start stub");
    }
}

pub struct Platform {
    audio: MacAudio,
    brightness: MacBrightness,
    media: MacMedia,
    input: MacInput,
    dial: MacDial,
    context: MacContext,
}

impl Platform {
    pub fn new() -> Self {
        Self {
            audio: MacAudio,
            brightness: MacBrightness,
            media: MacMedia,
            input: MacInput,
            dial: MacDial,
            context: MacContext,
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
