#![allow(non_snake_case, non_camel_case_types, non_upper_case_globals, dead_code)]
use std::ffi::c_void;

pub type HWND = *mut c_void;
pub type HINSTANCE = *mut c_void;
pub type HHOOK = *mut c_void;
pub type HANDLE = *mut c_void;
pub type DWORD = u32;
pub type BOOL = i32;
pub type LPARAM = isize;
pub type WPARAM = usize;
pub type LRESULT = isize;

pub const TRUE: BOOL = 1;
pub const FALSE: BOOL = 0;

pub const HWND_TOPMOST: HWND = (-1isize) as HWND;
pub const SWP_NOSIZE: DWORD = 0x0001;
pub const SWP_NOMOVE: DWORD = 0x0002;
pub const SWP_NOZORDER: DWORD = 0x0004;
pub const SWP_NOACTIVATE: DWORD = 0x0010;
pub const SWP_SHOWWINDOW: DWORD = 0x0040;

pub const WH_KEYBOARD_LL: i32 = 13;
pub const WM_KEYDOWN: u32 = 0x0100;
pub const WM_KEYUP: u32 = 0x0101;
pub const WM_SYSKEYDOWN: u32 = 0x0104;
pub const WM_SYSKEYUP: u32 = 0x0105;

pub const INPUT_MOUSE: DWORD = 0;
pub const INPUT_KEYBOARD: DWORD = 1;
pub const MOUSEEVENTF_WHEEL: DWORD = 0x0800;
pub const KEYEVENTF_EXTENDEDKEY: DWORD = 0x0001;
pub const KEYEVENTF_KEYUP: DWORD = 0x0002;
pub const KEYEVENTF_UNICODE: DWORD = 0x0004;
pub const KEYEVENTF_SCANCODE: DWORD = 0x0008;

pub const PROCESS_QUERY_LIMITED_INFORMATION: DWORD = 0x1000;

// Virtual-Key codes
pub const VK_BACK: u16 = 0x08;
pub const VK_TAB: u16 = 0x09;
pub const VK_RETURN: u16 = 0x0D;
pub const VK_SHIFT: u16 = 0x10;
pub const VK_CONTROL: u16 = 0x11;
pub const VK_MENU: u16 = 0x12; // Alt
pub const VK_PAUSE: u16 = 0x13;
pub const VK_CAPITAL: u16 = 0x14;
pub const VK_ESCAPE: u16 = 0x1B;
pub const VK_SPACE: u16 = 0x20;
pub const VK_PRIOR: u16 = 0x21; // Page Up
pub const VK_NEXT: u16 = 0x22;  // Page Down
pub const VK_END: u16 = 0x23;
pub const VK_HOME: u16 = 0x24;
pub const VK_LEFT: u16 = 0x25;
pub const VK_UP: u16 = 0x26;
pub const VK_RIGHT: u16 = 0x27;
pub const VK_DOWN: u16 = 0x28;
pub const VK_INSERT: u16 = 0x2D;
pub const VK_DELETE: u16 = 0x2E;
pub const VK_LWIN: u16 = 0x5B;
pub const VK_RWIN: u16 = 0x5C;

pub const VK_VOLUME_MUTE: u16 = 0xAD;
pub const VK_VOLUME_DOWN: u16 = 0xAE;
pub const VK_VOLUME_UP: u16 = 0xAF;
pub const VK_MEDIA_NEXT_TRACK: u16 = 0xB0;
pub const VK_MEDIA_PREV_TRACK: u16 = 0xB1;
pub const VK_MEDIA_STOP: u16 = 0xB2;
pub const VK_MEDIA_PLAY_PAUSE: u16 = 0xB3;

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct POINT {
    pub x: i32,
    pub y: i32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct MSG {
    pub hwnd: HWND,
    pub message: u32,
    pub wParam: WPARAM,
    pub lParam: LPARAM,
    pub time: DWORD,
    pub pt: POINT,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct KBDLLHOOKSTRUCT {
    pub vkCode: DWORD,
    pub scanCode: DWORD,
    pub flags: DWORD,
    pub time: DWORD,
    pub dwExtraInfo: usize,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct MOUSEINPUT {
    pub dx: i32,
    pub dy: i32,
    pub mouseData: DWORD,
    pub dwFlags: DWORD,
    pub time: DWORD,
    pub dwExtraInfo: usize,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct KEYBDINPUT {
    pub wVk: u16,
    pub wScan: u16,
    pub dwFlags: DWORD,
    pub time: DWORD,
    pub dwExtraInfo: usize,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub union INPUT_UNION {
    pub mi: MOUSEINPUT,
    pub ki: KEYBDINPUT,
    pub padding: [u8; 32],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct INPUT {
    pub type_: DWORD,
    pub u: INPUT_UNION,
}

pub type HOOKPROC = Option<unsafe extern "system" fn(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT>;
pub type HRESULT = i32;
pub type HMONITOR = *mut c_void;
pub type HDC = *mut c_void;

pub const SYNTHETIC_INPUT_TAG: usize = 0x4449414C; // "DIAL" in ASCII
pub const SW_RESTORE: i32 = 9;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct GUID {
    pub data1: u32,
    pub data2: u16,
    pub data3: u16,
    pub data4: [u8; 8],
}

pub const CLSID_MMDEVICE_ENUMERATOR: GUID = GUID {
    data1: 0xBCDE0395,
    data2: 0xE52F,
    data3: 0x467C,
    data4: [0x8E, 0x3D, 0xC4, 0x57, 0x92, 0x91, 0x69, 0x2E],
};

pub const IID_IMMDEVICE_ENUMERATOR: GUID = GUID {
    data1: 0xA95664D2,
    data2: 0x9614,
    data3: 0x4F35,
    data4: [0xA7, 0x46, 0xDE, 0x8D, 0xB6, 0x36, 0x17, 0xE6],
};

pub const IID_IAUDIO_ENDPOINT_VOLUME: GUID = GUID {
    data1: 0x5CDF2C82,
    data2: 0x841E,
    data3: 0x4546,
    data4: [0x97, 0x22, 0x0C, 0xF7, 0x40, 0x78, 0x22, 0x9A],
};

#[repr(C)]
#[derive(Clone, Copy)]
pub struct PHYSICAL_MONITOR {
    pub hPhysicalMonitor: HANDLE,
    pub szPhysicalMonitorDescription: [u16; 128],
}

pub type MONITORENUMPROC = Option<unsafe extern "system" fn(hMonitor: HMONITOR, hdcMonitor: HDC, lprcMonitor: *mut c_void, dwData: LPARAM) -> BOOL>;

#[link(name = "user32")]
extern "system" {
    pub fn GetForegroundWindow() -> HWND;
    pub fn SetForegroundWindow(hWnd: HWND) -> BOOL;
    pub fn SetWindowPos(
        hWnd: HWND,
        hWndInsertAfter: HWND,
        X: i32,
        Y: i32,
        cx: i32,
        cy: i32,
        uFlags: DWORD,
    ) -> BOOL;
    pub fn ShowWindow(hWnd: HWND, nCmdShow: i32) -> BOOL;
    pub fn GetWindowTextW(hWnd: HWND, lpString: *mut u16, nMaxCount: i32) -> i32;
    pub fn GetClassNameW(hWnd: HWND, lpClassName: *mut u16, nMaxCount: i32) -> i32;
    pub fn GetWindowThreadProcessId(hWnd: HWND, lpdwProcessId: *mut DWORD) -> DWORD;
    pub fn SetWindowsHookExW(idHook: i32, lpfn: HOOKPROC, hmod: HINSTANCE, dwThreadId: DWORD) -> HHOOK;
    pub fn UnhookWindowsHookEx(hhk: HHOOK) -> BOOL;
    pub fn CallNextHookEx(hhk: HHOOK, nCode: i32, wParam: WPARAM, lParam: LPARAM) -> LRESULT;
    pub fn GetMessageW(lpMsg: *mut MSG, hWnd: HWND, wMsgFilterMin: u32, wMsgFilterMax: u32) -> BOOL;
    pub fn TranslateMessage(lpMsg: *const MSG) -> BOOL;
    pub fn DispatchMessageW(lpMsg: *const MSG) -> LRESULT;
    pub fn PostQuitMessage(nExitCode: i32);
    pub fn SendInput(cInputs: u32, pInputs: *const INPUT, cbSize: i32) -> u32;
    pub fn EnumDisplayMonitors(hdc: HDC, lprcClip: *const c_void, lpfnEnum: MONITORENUMPROC, dwData: LPARAM) -> BOOL;
}

#[link(name = "kernel32")]
extern "system" {
    pub fn GetCurrentProcessId() -> DWORD;
    pub fn OpenProcess(dwDesiredAccess: DWORD, bInheritHandle: BOOL, dwProcessId: DWORD) -> HANDLE;
    pub fn CloseHandle(hObject: HANDLE) -> BOOL;
    pub fn QueryFullProcessImageNameW(hProcess: HANDLE, dwFlags: DWORD, lpExeName: *mut u16, lpdwSize: *mut DWORD) -> BOOL;
    pub fn GetModuleHandleW(lpModuleName: *const u16) -> HINSTANCE;
    pub fn LoadLibraryW(lpLibFileName: *const u16) -> HINSTANCE;
    pub fn GetProcAddress(hModule: HINSTANCE, lpProcName: *const u8) -> *const c_void;
    pub fn FreeLibrary(hLibModule: HINSTANCE) -> BOOL;
}

#[link(name = "ole32")]
extern "system" {
    pub fn CoInitializeEx(pvReserved: *mut c_void, dwCoInit: DWORD) -> HRESULT;
    pub fn CoUninitialize();
    pub fn CoCreateInstance(
        rclsid: *const GUID,
        pUnkOuter: *mut c_void,
        dwClsContext: DWORD,
        riid: *const GUID,
        ppv: *mut *mut c_void,
    ) -> HRESULT;
}

// Dynamic function pointer definitions for dxva2.dll (loaded dynamically via LoadLibraryW to prevent startup loader crashes)
pub type FnGetNumberOfPhysicalMonitorsFromHMONITOR = unsafe extern "system" fn(
    hMonitor: HMONITOR,
    pdwNumberOfPhysicalMonitors: *mut DWORD,
) -> BOOL;

pub type FnGetPhysicalMonitorsFromHMONITOR = unsafe extern "system" fn(
    hMonitor: HMONITOR,
    dwPhysicalMonitorArraySize: DWORD,
    pPhysicalMonitorArray: *mut PHYSICAL_MONITOR,
) -> BOOL;

pub type FnDestroyPhysicalMonitors = unsafe extern "system" fn(
    dwPhysicalMonitorArraySize: DWORD,
    pPhysicalMonitorArray: *mut PHYSICAL_MONITOR,
) -> BOOL;

pub type FnGetMonitorBrightness = unsafe extern "system" fn(
    hMonitor: HANDLE,
    pdwMinimumBrightness: *mut DWORD,
    pdwCurrentBrightness: *mut DWORD,
    pdwMaximumBrightness: *mut DWORD,
) -> BOOL;

pub type FnSetMonitorBrightness = unsafe extern "system" fn(
    hMonitor: HANDLE,
    dwNewBrightness: DWORD,
) -> BOOL;
