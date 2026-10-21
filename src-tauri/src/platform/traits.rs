use std::sync::atomic::{AtomicBool, AtomicI32};
use std::sync::Arc;
use tauri::AppHandle;

/// Abstract interface for system volume querying and adjustment.
pub trait SystemAudio: Send + Sync {
    fn get_volume(&self) -> i32;
    fn set_volume(&self, pct: i32) -> i32;
    fn toggle_mute(&self);
}

/// Abstract interface for screen backlight brightness control.
pub trait SystemBrightness: Send + Sync {
    fn get_brightness(&self) -> i32;
    fn set_brightness(&self, pct: i32) -> i32;
    fn step_brightness(&self, direction: i32);
}

/// Abstract interface for playback media commands (MPRIS / OS media sessions).
pub trait MediaControl: Send + Sync {
    fn play_pause(&self);
    fn next_track(&self);
    fn prev_track(&self);
}

/// Abstract interface for keyboard emulation (hotkeys, shortcuts).
pub trait VirtualInput: Send + Sync {
    fn emit_hotkey(&self, keys: &[String]);
}

/// Abstract interface for rotary dial hardware listeners and event interceptors.
pub trait DialBackend: Send + Sync {
    fn start_listeners(
        &self,
        app: AppHandle,
        is_hud_open: Arc<AtomicBool>,
        locked_vol: Arc<AtomicI32>,
    );
}

/// Abstract interface for foreground active window context tracking.
pub trait ContextDetector: Send + Sync {
    fn start(&self, app: AppHandle);
}

/// Unified platform provider trait implemented by each OS backend.
pub trait PlatformProvider: Send + Sync {
    fn audio(&self) -> &dyn SystemAudio;
    fn brightness(&self) -> &dyn SystemBrightness;
    fn media(&self) -> &dyn MediaControl;
    fn input(&self) -> &dyn VirtualInput;
    fn dial(&self) -> &dyn DialBackend;
    fn context(&self) -> &dyn ContextDetector;
}
