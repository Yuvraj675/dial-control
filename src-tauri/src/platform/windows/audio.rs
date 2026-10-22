use crate::platform::traits::SystemAudio;
use crate::platform::windows::ffi::*;
use std::ffi::c_void;
use std::ptr::{null, null_mut};
use std::sync::atomic::{AtomicI32, Ordering};

#[repr(C)]
struct IUnknownVtbl {
    QueryInterface: *const c_void,
    AddRef: unsafe extern "system" fn(this: *mut c_void) -> u32,
    Release: unsafe extern "system" fn(this: *mut c_void) -> u32,
}

#[repr(C)]
struct IMMDeviceEnumeratorVtbl {
    base: IUnknownVtbl,
    EnumAudioEndpoints: *const c_void,
    GetDefaultAudioEndpoint: unsafe extern "system" fn(
        this: *mut c_void,
        dataFlow: i32,
        role: i32,
        ppEndpoint: *mut *mut c_void,
    ) -> HRESULT,
    GetDevice: *const c_void,
    RegisterEndpointNotificationCallback: *const c_void,
    UnregisterEndpointNotificationCallback: *const c_void,
}

#[repr(C)]
struct IMMDeviceVtbl {
    base: IUnknownVtbl,
    Activate: unsafe extern "system" fn(
        this: *mut c_void,
        iid: *const GUID,
        dwClsCtx: DWORD,
        pActivationParams: *mut c_void,
        ppInterface: *mut *mut c_void,
    ) -> HRESULT,
    OpenPropertyStore: *const c_void,
    GetId: *const c_void,
    GetState: *const c_void,
}

#[repr(C)]
struct IAudioEndpointVolumeVtbl {
    base: IUnknownVtbl,
    RegisterControlChangeNotify: *const c_void,
    UnregisterControlChangeNotify: *const c_void,
    GetChannelCount: *const c_void,
    SetMasterVolumeLevel: *const c_void,
    SetMasterVolumeLevelScalar: unsafe extern "system" fn(
        this: *mut c_void,
        fLevel: f32,
        pguidEventContext: *const GUID,
    ) -> HRESULT,
    GetMasterVolumeLevel: *const c_void,
    GetMasterVolumeLevelScalar: unsafe extern "system" fn(
        this: *mut c_void,
        pfLevel: *mut f32,
    ) -> HRESULT,
    SetChannelVolumeLevel: *const c_void,
    SetChannelVolumeLevelScalar: *const c_void,
    GetChannelVolumeLevel: *const c_void,
    GetChannelVolumeLevelScalar: *const c_void,
    SetMute: unsafe extern "system" fn(
        this: *mut c_void,
        bMute: BOOL,
        pguidEventContext: *const GUID,
    ) -> HRESULT,
    GetMute: unsafe extern "system" fn(
        this: *mut c_void,
        pbMute: *mut BOOL,
    ) -> HRESULT,
    GetVolumeStepInfo: *const c_void,
    VolumeStepUp: *const c_void,
    VolumeStepDown: *const c_void,
    QueryHardwareSupport: *const c_void,
    GetVolumeRange: *const c_void,
}

unsafe fn with_endpoint_volume<F, R>(f: F) -> Option<R>
where
    F: FnOnce(*mut *const IAudioEndpointVolumeVtbl) -> Option<R>,
{
    let _ = CoInitializeEx(null_mut(), 0);

    let mut enumerator: *mut *const IMMDeviceEnumeratorVtbl = null_mut();
    let hr = CoCreateInstance(
        &CLSID_MMDEVICE_ENUMERATOR,
        null_mut(),
        23, // CLSCTX_ALL
        &IID_IMMDEVICE_ENUMERATOR,
        &mut enumerator as *mut _ as *mut *mut c_void,
    );
    if hr != 0 || enumerator.is_null() {
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

    if hr != 0 || device.is_null() {
        return None;
    }

    let mut volume: *mut *const IAudioEndpointVolumeVtbl = null_mut();
    let hr = ((**device).Activate)(
        device as *mut c_void,
        &IID_IAUDIO_ENDPOINT_VOLUME,
        23,
        null_mut(),
        &mut volume as *mut _ as *mut *mut c_void,
    );

    ((**device).base.Release)(device as *mut c_void);

    if hr != 0 || volume.is_null() {
        return None;
    }

    let result = f(volume);

    ((**volume).base.Release)(volume as *mut c_void);

    result
}

fn query_hardware_volume() -> Option<i32> {
    unsafe {
        with_endpoint_volume(|vol| {
            let mut scalar: f32 = 0.0;
            let hr = ((**vol).GetMasterVolumeLevelScalar)(vol as *mut c_void, &mut scalar);
            if hr == 0 {
                Some((scalar * 100.0).round() as i32)
            } else {
                None
            }
        })
    }
}

fn set_hardware_volume(pct: i32) -> bool {
    let scalar = (pct as f32 / 100.0).clamp(0.0, 1.0);
    unsafe {
        with_endpoint_volume(|vol| {
            let hr = ((**vol).SetMasterVolumeLevelScalar)(vol as *mut c_void, scalar, null());
            Some(hr == 0)
        })
        .unwrap_or(false)
    }
}

fn toggle_hardware_mute() {
    unsafe {
        with_endpoint_volume(|vol| {
            let mut muted: BOOL = 0;
            if ((**vol).GetMute)(vol as *mut c_void, &mut muted) == 0 {
                let next_mute = if muted != 0 { 0 } else { 1 };
                let _ = ((**vol).SetMute)(vol as *mut c_void, next_mute, null());
            }
            Some(())
        });
    }
}

pub struct WindowsAudio {
    cached_vol: AtomicI32,
}

impl WindowsAudio {
    pub fn new() -> Self {
        let init_vol = query_hardware_volume().unwrap_or(50);
        println!("[Windows Audio] Initial system volume detected: {}%", init_vol);
        Self {
            cached_vol: AtomicI32::new(init_vol),
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
        set_hardware_volume(target);
        println!("[Windows Audio] Master volume set to {}%", target);
        target
    }

    fn toggle_mute(&self) {
        println!("[Windows Audio] Master mute toggled");
        toggle_hardware_mute();
    }
}
