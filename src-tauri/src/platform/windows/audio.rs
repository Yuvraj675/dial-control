#![allow(non_snake_case, non_camel_case_types, non_upper_case_globals, dead_code)]
use crate::platform::traits::SystemAudio;
use crate::platform::windows::ffi::*;
use std::ffi::c_void;
use std::ptr::{null, null_mut};
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::mpsc::{channel, Sender};
use std::sync::Arc;
use std::thread;

pub unsafe fn with_endpoint_volume<F, R>(f: F) -> Option<R>
where
    F: FnOnce(*mut *const IAudioEndpointVolumeVtbl) -> Option<R>,
{
    let hr_init = CoInitializeEx(null_mut(), 0);
    let com_ok = hr_init == 0 || hr_init == 1;

    let mut enumerator: *mut *const IMMDeviceEnumeratorVtbl = null_mut();
    let hr = CoCreateInstance(
        &CLSID_MMDEVICE_ENUMERATOR,
        null_mut(),
        CLSCTX_ALL,
        &IID_IMMDEVICE_ENUMERATOR,
        &mut enumerator as *mut _ as *mut *mut c_void,
    );
    if hr < 0 || enumerator.is_null() {
        if com_ok {
            CoUninitialize();
        }
        return None;
    }

    let mut device: *mut *const IMMDeviceVtbl = null_mut();
    // eRender = 0, eMultimedia = 1
    let hr = ((**enumerator).GetDefaultAudioEndpoint)(
        enumerator as *mut c_void,
        0,
        1,
        &mut device as *mut _ as *mut *mut c_void,
    );

    ((**enumerator).base.Release)(enumerator as *mut c_void);

    if hr < 0 || device.is_null() {
        if com_ok {
            CoUninitialize();
        }
        return None;
    }

    let mut volume: *mut *const IAudioEndpointVolumeVtbl = null_mut();
    let hr = ((**device).Activate)(
        device as *mut c_void,
        &IID_IAUDIO_ENDPOINT_VOLUME,
        CLSCTX_ALL,
        null_mut(),
        &mut volume as *mut _ as *mut *mut c_void,
    );

    ((**device).base.Release)(device as *mut c_void);

    if hr < 0 || volume.is_null() {
        if com_ok {
            CoUninitialize();
        }
        return None;
    }

    let result = f(volume);

    ((**volume).base.Release)(volume as *mut c_void);

    if com_ok {
        CoUninitialize();
    }

    result
}

pub fn query_hardware_volume() -> Option<i32> {
    unsafe {
        with_endpoint_volume(|vol| {
            let mut scalar: f32 = 0.0;
            let hr = ((**vol).GetMasterVolumeLevelScalar)(vol as *mut c_void, &mut scalar);
            if hr >= 0 {
                Some((scalar * 100.0).round() as i32)
            } else {
                None
            }
        })
    }
}

pub fn set_hardware_volume(pct: i32) -> bool {
    set_hardware_volume_with_context(pct, &DIAL_CONTROL_EVENT_CONTEXT)
}

pub fn set_hardware_volume_with_context(pct: i32, pguid: *const GUID) -> bool {
    let scalar = (pct as f32 / 100.0).clamp(0.0, 1.0);
    unsafe {
        with_endpoint_volume(|vol| {
            let hr = ((**vol).SetMasterVolumeLevelScalar)(vol as *mut c_void, scalar, pguid);
            Some(hr >= 0)
        })
        .unwrap_or(false)
    }
}

fn toggle_hardware_mute() {
    unsafe {
        with_endpoint_volume(|vol| {
            let mut muted: BOOL = 0;
            if ((**vol).GetMute)(vol as *mut c_void, &mut muted) >= 0 {
                let next_mute = if muted != 0 { 0 } else { 1 };
                let _ = ((**vol).SetMute)(vol as *mut c_void, next_mute, null());
            }
            Some(())
        });
    }
}

enum AudioCommand {
    SetVolume(i32),
    ToggleMute,
    RefreshVolume,
}

#[derive(Clone)]
pub struct WindowsAudio {
    cached_vol: Arc<AtomicI32>,
    cmd_tx: Sender<AudioCommand>,
}

impl WindowsAudio {
    pub fn new() -> Self {
        let cached = Arc::new(AtomicI32::new(50));
        let cached_clone = cached.clone();
        let (tx, rx) = channel::<AudioCommand>();

        // Spawn a single dedicated COM worker thread.
        // This ensures the main UI thread's COM apartment mode is never polluted,
        // preventing WebView2 crashes on Windows startup.
        thread::spawn(move || {
            let hr_init = unsafe { CoInitializeEx(null_mut(), 0) };
            let com_ok = hr_init == 0 || hr_init == 1;

            // Micro-delay so default audio endpoints are enumerated
            thread::sleep(std::time::Duration::from_millis(50));

            if let Some(vol) = query_hardware_volume() {
                println!("[Windows Audio] Initial system volume detected: {}%", vol);
                cached_clone.store(vol, Ordering::SeqCst);
            }

            while let Ok(cmd) = rx.recv() {
                match cmd {
                    AudioCommand::SetVolume(mut target) => {
                        // Coalesce rapid volume updates from fast rotary turns
                        while let Ok(next_cmd) = rx.try_recv() {
                            match next_cmd {
                                AudioCommand::SetVolume(t) => target = t,
                                AudioCommand::ToggleMute => toggle_hardware_mute(),
                                AudioCommand::RefreshVolume => {}
                            }
                        }
                        set_hardware_volume(target);
                    }
                    AudioCommand::ToggleMute => {
                        toggle_hardware_mute();
                    }
                    AudioCommand::RefreshVolume => {
                        if let Some(vol) = query_hardware_volume() {
                            cached_clone.store(vol, Ordering::SeqCst);
                        }
                    }
                }
            }

            if com_ok {
                unsafe {
                    CoUninitialize();
                }
            }
        });

        Self {
            cached_vol: cached,
            cmd_tx: tx,
        }
    }
}

impl SystemAudio for WindowsAudio {
    fn get_volume(&self) -> i32 {
        if let Some(vol) = query_hardware_volume() {
            self.cached_vol.store(vol, Ordering::SeqCst);
            vol
        } else {
            self.cached_vol.load(Ordering::SeqCst)
        }
    }

    fn set_volume(&self, pct: i32) -> i32 {
        let target = pct.clamp(0, 100);
        self.cached_vol.store(target, Ordering::SeqCst);
        let _ = self.cmd_tx.send(AudioCommand::SetVolume(target));
        println!("[Windows Audio] Master volume target: {}%", target);
        target
    }

    fn toggle_mute(&self) {
        println!("[Windows Audio] Master mute toggled");
        let _ = self.cmd_tx.send(AudioCommand::ToggleMute);
    }
}
