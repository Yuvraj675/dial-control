use crate::platform::traits::SystemAudio;
use std::process::Command;

#[cfg(target_os = "macos")]
use super::ffi::*;

pub struct MacAudio;

impl MacAudio {
    pub fn new() -> Self {
        Self
    }

    #[cfg(target_os = "macos")]
    unsafe fn get_default_output_device() -> Option<AudioObjectID> {
        let mut device_id: AudioObjectID = 0;
        let mut size = std::mem::size_of::<AudioObjectID>() as u32;
        let address = AudioObjectPropertyAddress {
            mSelector: kAudioHardwarePropertyDefaultOutputDevice,
            mScope: kAudioObjectPropertyScopeGlobal,
            mElement: kAudioObjectPropertyElementMain,
        };

        let status = AudioObjectGetPropertyData(
            kAudioObjectSystemObject,
            &address,
            0,
            std::ptr::null(),
            &mut size,
            &mut device_id as *mut _ as *mut std::ffi::c_void,
        );

        if status == 0 && device_id != 0 {
            Some(device_id)
        } else {
            None
        }
    }
}

impl SystemAudio for MacAudio {
    fn get_volume(&self) -> i32 {
        #[cfg(target_os = "macos")]
        unsafe {
            if let Some(device_id) = Self::get_default_output_device() {
                let mut vol: f32 = 0.0;
                let mut size = std::mem::size_of::<f32>() as u32;

                // 1. Try Virtual Master Volume first
                let address_vmvc = AudioObjectPropertyAddress {
                    mSelector: kAudioHardwareServiceDeviceProperty_VirtualMasterVolume,
                    mScope: kAudioObjectPropertyScopeOutput,
                    mElement: kAudioObjectPropertyElementMain,
                };

                let status = AudioObjectGetPropertyData(
                    device_id,
                    &address_vmvc,
                    0,
                    std::ptr::null(),
                    &mut size,
                    &mut vol as *mut _ as *mut std::ffi::c_void,
                );

                if status == 0 {
                    return ((vol * 100.0).round() as i32).clamp(0, 100);
                }

                // 2. Try Scalar Volume
                let address_volm = AudioObjectPropertyAddress {
                    mSelector: kAudioDevicePropertyVolumeScalar,
                    mScope: kAudioObjectPropertyScopeOutput,
                    mElement: kAudioObjectPropertyElementMain,
                };

                let status = AudioObjectGetPropertyData(
                    device_id,
                    &address_volm,
                    0,
                    std::ptr::null(),
                    &mut size,
                    &mut vol as *mut _ as *mut std::ffi::c_void,
                );

                if status == 0 {
                    return ((vol * 100.0).round() as i32).clamp(0, 100);
                }
            }
        }

        // Fallback: osascript
        if let Ok(output) = Command::new("osascript")
            .args(["-e", "output volume of (get volume settings)"])
            .output()
        {
            if let Ok(text) = String::from_utf8(output.stdout) {
                if let Ok(val) = text.trim().parse::<i32>() {
                    return val.clamp(0, 100);
                }
            }
        }

        50
    }

    fn set_volume(&self, pct: i32) -> i32 {
        let pct = pct.clamp(0, 100);
        let scalar = (pct as f32) / 100.0;

        #[cfg(target_os = "macos")]
        unsafe {
            if let Some(device_id) = Self::get_default_output_device() {
                let size = std::mem::size_of::<f32>() as u32;

                // 1. Set Virtual Master Volume
                let address_vmvc = AudioObjectPropertyAddress {
                    mSelector: kAudioHardwareServiceDeviceProperty_VirtualMasterVolume,
                    mScope: kAudioObjectPropertyScopeOutput,
                    mElement: kAudioObjectPropertyElementMain,
                };

                let status = AudioObjectSetPropertyData(
                    device_id,
                    &address_vmvc,
                    0,
                    std::ptr::null(),
                    size,
                    &scalar as *const _ as *const std::ffi::c_void,
                );

                if status == 0 {
                    println!("[macOS Volume] Set to {}% via CoreAudio", pct);
                    return pct;
                }

                // 2. Set Scalar Volume
                let address_volm = AudioObjectPropertyAddress {
                    mSelector: kAudioDevicePropertyVolumeScalar,
                    mScope: kAudioObjectPropertyScopeOutput,
                    mElement: kAudioObjectPropertyElementMain,
                };

                let status = AudioObjectSetPropertyData(
                    device_id,
                    &address_volm,
                    0,
                    std::ptr::null(),
                    size,
                    &scalar as *const _ as *const std::ffi::c_void,
                );

                if status == 0 {
                    println!("[macOS Volume] Set to {}% via CoreAudio Scalar", pct);
                    return pct;
                }
            }
        }

        // Fallback: osascript
        let script = format!("set volume output volume {}", pct);
        let _ = Command::new("osascript").args(["-e", &script]).status();
        println!("[macOS Volume] Set to {}% via osascript", pct);
        pct
    }

    fn toggle_mute(&self) {
        #[cfg(target_os = "macos")]
        unsafe {
            if let Some(device_id) = Self::get_default_output_device() {
                let mut muted: u32 = 0;
                let mut size = std::mem::size_of::<u32>() as u32;
                let address = AudioObjectPropertyAddress {
                    mSelector: kAudioDevicePropertyMute,
                    mScope: kAudioObjectPropertyScopeOutput,
                    mElement: kAudioObjectPropertyElementMain,
                };

                let get_status = AudioObjectGetPropertyData(
                    device_id,
                    &address,
                    0,
                    std::ptr::null(),
                    &mut size,
                    &mut muted as *mut _ as *mut std::ffi::c_void,
                );

                if get_status == 0 {
                    let new_mute: u32 = if muted == 0 { 1 } else { 0 };
                    let _ = AudioObjectSetPropertyData(
                        device_id,
                        &address,
                        0,
                        std::ptr::null(),
                        size,
                        &new_mute as *const _ as *const std::ffi::c_void,
                    );
                    println!("[macOS Audio] Toggled mute via CoreAudio: {}", new_mute == 1);
                    return;
                }
            }
        }

        // Fallback: osascript
        let _ = Command::new("osascript")
            .args(["-e", "set volume output muted not (output muted of (get volume settings))"])
            .status();
        println!("[macOS Audio] Toggled mute via osascript");
    }
}
