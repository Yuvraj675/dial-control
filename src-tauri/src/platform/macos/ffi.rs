#![allow(non_snake_case, non_camel_case_types, non_upper_case_globals, dead_code)]

use std::ffi::c_void;

// CoreFoundation Types
pub type CFTypeRef = *const c_void;
pub type CFStringRef = *const c_void;
pub type CFIndex = isize;
pub type CFAllocatorRef = *const c_void;
pub type CFRunLoopRef = *mut c_void;
pub type CFRunLoopSourceRef = *mut c_void;
pub type CFRunLoopMode = CFStringRef;
pub type CFMachPortRef = *mut c_void;
pub type Boolean = u8;

// CoreGraphics Event Tap Types
pub type CGEventRef = *mut c_void;
pub type CGEventTapProxy = *mut c_void;
pub type CGEventType = u32;
pub type CGEventMask = u64;

pub type CGEventTapLocation = u32;
pub const kCGHIDEventTap: CGEventTapLocation = 0;
pub const kCGSessionEventTap: CGEventTapLocation = 1;
pub const kCGAnnotatedSessionEventTap: CGEventTapLocation = 2;

pub type CGEventTapPlacement = u32;
pub const kCGHeadInsertEventTap: CGEventTapPlacement = 0;
pub const kCGTailAppendEventTap: CGEventTapPlacement = 1;

pub type CGEventTapOptions = u32;
pub const kCGEventTapOptionDefault: CGEventTapOptions = 0;
pub const kCGEventTapOptionListenOnly: CGEventTapOptions = 1;

pub const kCGEventNull: CGEventType = 0;
pub const kCGEventLeftMouseDown: CGEventType = 1;
pub const kCGEventLeftMouseUp: CGEventType = 2;
pub const kCGEventRightMouseDown: CGEventType = 3;
pub const kCGEventRightMouseUp: CGEventType = 4;
pub const kCGEventMouseMoved: CGEventType = 5;
pub const kCGEventKeyDown: CGEventType = 10;
pub const kCGEventKeyUp: CGEventType = 11;
pub const kCGEventFlagsChanged: CGEventType = 12;
pub const kCGEventScrollWheel: CGEventType = 22;
pub const NX_SYSDEFINED: CGEventType = 14;

// Event tap disabled notifications
pub const kCGEventTapDisabledByTimeout: CGEventType = 0xFFFFFFFE;
pub const kCGEventTapDisabledByUserInput: CGEventType = 0xFFFFFFFF;

// Scroll wheel units
pub const kCGScrollEventUnitPixel: u32 = 0;
pub const kCGScrollEventUnitLine: u32 = 1;

// System defined subtypes & auxiliary key definitions
pub const NX_SUBTYPE_AUX_CONTROL_BUTTONS: i16 = 8;

// CGEventField: pid of the process that posted the event
pub const kCGEventSourceUnixProcessID: u32 = 41;

pub const NX_KEYTYPE_SOUND_UP: u32 = 0;
pub const NX_KEYTYPE_SOUND_DOWN: u32 = 1;
pub const NX_KEYTYPE_BRIGHTNESS_UP: u32 = 2;
pub const NX_KEYTYPE_BRIGHTNESS_DOWN: u32 = 3;
pub const NX_KEYTYPE_MUTE: u32 = 7;
pub const NX_KEYTYPE_PLAY: u32 = 16;
pub const NX_KEYTYPE_NEXT: u32 = 17;
pub const NX_KEYTYPE_PREVIOUS: u32 = 18;
pub const NX_KEYTYPE_FAST: u32 = 19;
pub const NX_KEYTYPE_REWIND: u32 = 20;

// Modifier flags
pub const kCGEventFlagMaskAlphaShift: u64 = 0x00010000;
pub const kCGEventFlagMaskShift: u64 = 0x00020000;
pub const kCGEventFlagMaskControl: u64 = 0x00040000;
pub const kCGEventFlagMaskAlternate: u64 = 0x00080000;
pub const kCGEventFlagMaskCommand: u64 = 0x00100000;
pub const kCGEventFlagMaskSecondaryFn: u64 = 0x00800000;

pub type CGEventTapCallBack = unsafe extern "C" fn(
    proxy: CGEventTapProxy,
    eventType: CGEventType,
    event: CGEventRef,
    userInfo: *mut c_void,
) -> CGEventRef;

// CoreAudio Types & Constants
pub type AudioObjectID = u32;
pub type AudioObjectPropertySelector = u32;
pub type AudioObjectPropertyScope = u32;
pub type AudioObjectPropertyElement = u32;
pub type OSStatus = i32;

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct AudioObjectPropertyAddress {
    pub mSelector: AudioObjectPropertySelector,
    pub mScope: AudioObjectPropertyScope,
    pub mElement: AudioObjectPropertyElement,
}

pub const kAudioObjectSystemObject: AudioObjectID = 1;
pub const kAudioObjectPropertyScopeGlobal: AudioObjectPropertyScope = 0x676C6F62; // 'glob'
pub const kAudioObjectPropertyScopeOutput: AudioObjectPropertyScope = 0x6F757470; // 'outp'
pub const kAudioObjectPropertyElementMain: AudioObjectPropertyElement = 0;

pub const kAudioHardwarePropertyDefaultOutputDevice: AudioObjectPropertySelector = 0x644F7574; // 'dOut'
pub const kAudioHardwareServiceDeviceProperty_VirtualMasterVolume: AudioObjectPropertySelector = 0x766D7663; // 'vmvc'
pub const kAudioDevicePropertyVolumeScalar: AudioObjectPropertySelector = 0x766F6C6D; // 'volm'
pub const kAudioDevicePropertyMute: AudioObjectPropertySelector = 0x6D757465; // 'mute'

#[link(name = "CoreFoundation", kind = "framework")]
extern "C" {
    pub static kCFRunLoopCommonModes: CFRunLoopMode;
    pub static kCFRunLoopDefaultMode: CFRunLoopMode;

    pub fn CFRunLoopGetCurrent() -> CFRunLoopRef;
    pub fn CFRunLoopRun();
    pub fn CFRunLoopStop(rl: CFRunLoopRef);
    pub fn CFRunLoopAddSource(rl: CFRunLoopRef, source: CFRunLoopSourceRef, mode: CFRunLoopMode);
    pub fn CFMachPortCreateRunLoopSource(
        allocator: CFAllocatorRef,
        port: CFMachPortRef,
        order: CFIndex,
    ) -> CFRunLoopSourceRef;
    pub fn CFRelease(cf: CFTypeRef);
}

#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    pub fn CGEventTapCreate(
        tap: CGEventTapLocation,
        place: CGEventTapPlacement,
        options: CGEventTapOptions,
        eventsOfInterest: CGEventMask,
        callback: CGEventTapCallBack,
        userInfo: *mut c_void,
    ) -> CFMachPortRef;

    pub fn CGEventTapEnable(port: CFMachPortRef, enable: bool);
    pub fn CGEventPost(tap: CGEventTapLocation, event: CGEventRef);
    pub fn CGEventCreateScrollWheelEvent2(
        source: *mut c_void,
        units: u32,
        wheelCount: u32,
        wheel1: i32,
        wheel2: i32,
        wheel3: i32,
    ) -> CGEventRef;
    pub fn CGEventCreateKeyboardEvent(
        source: *mut c_void,
        virtualKey: u16,
        keyDown: bool,
    ) -> CGEventRef;
    pub fn CGEventSetFlags(event: CGEventRef, flags: u64);
    pub fn CGEventKeyboardSetUnicodeString(event: CGEventRef, length: usize, string: *const u16);
    pub fn CGEventGetIntegerValueField(event: CGEventRef, field: u32) -> i64;
    pub fn CGMainDisplayID() -> u32;
}

#[link(name = "CoreAudio", kind = "framework")]
extern "C" {
    pub fn AudioObjectGetPropertyData(
        inObjectID: AudioObjectID,
        inAddress: *const AudioObjectPropertyAddress,
        inQualifierDataSize: u32,
        inQualifierData: *const c_void,
        ioDataSize: *mut u32,
        outData: *mut c_void,
    ) -> OSStatus;

    pub fn AudioObjectSetPropertyData(
        inObjectID: AudioObjectID,
        inAddress: *const AudioObjectPropertyAddress,
        inQualifierDataSize: u32,
        inQualifierData: *const c_void,
        inDataSize: u32,
        inData: *const c_void,
    ) -> OSStatus;

    pub fn AudioObjectHasProperty(
        inObjectID: AudioObjectID,
        inAddress: *const AudioObjectPropertyAddress,
    ) -> Boolean;

    pub fn AudioObjectAddPropertyListener(
        inObjectID: AudioObjectID,
        inAddress: *const AudioObjectPropertyAddress,
        inListener: AudioObjectPropertyListenerProc,
        inClientData: *mut c_void,
    ) -> OSStatus;

    pub fn AudioObjectRemovePropertyListener(
        inObjectID: AudioObjectID,
        inAddress: *const AudioObjectPropertyAddress,
        inListener: AudioObjectPropertyListenerProc,
        inClientData: *mut c_void,
    ) -> OSStatus;
}

pub type AudioObjectPropertyListenerProc = unsafe extern "C" fn(
    inObjectID: AudioObjectID,
    inNumberAddresses: u32,
    inAddresses: *const AudioObjectPropertyAddress,
    inClientData: *mut c_void,
) -> OSStatus;

extern "C" {
    pub fn dlopen(filename: *const std::ffi::c_char, flag: std::ffi::c_int) -> *mut c_void;
    pub fn dlsym(handle: *mut c_void, symbol: *const std::ffi::c_char) -> *mut c_void;
    pub fn dlclose(handle: *mut c_void) -> std::ffi::c_int;
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct CGPoint {
    pub x: f64,
    pub y: f64,
}
pub type NSPoint = CGPoint;

#[link(name = "AppKit", kind = "framework")]
#[link(name = "objc", kind = "dylib")]
extern "C" {
    pub fn objc_getClass(name: *const std::ffi::c_char) -> *mut c_void;
    pub fn sel_registerName(name: *const std::ffi::c_char) -> *mut c_void;
    pub fn objc_msgSend();
    pub fn objc_autoreleasePoolPush() -> *mut c_void;
    pub fn objc_autoreleasePoolPop(pool: *mut c_void);
}
