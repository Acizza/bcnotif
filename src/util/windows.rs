extern crate winapi;
extern crate user32;
extern crate kernel32;

use std::ffi::OsStr;
use std::mem;
use std::os::windows::ffi::OsStrExt;
use std::ptr;
use self::winapi::winuser::{WNDCLASSW, HWND_MESSAGE, WM_USER, WM_DESTROY};
use self::winapi::minwindef::{DWORD, UINT, LRESULT, LPARAM, WPARAM};
use self::winapi::windef::{HWND, HICON};
use self::winapi::winnt::WCHAR;
use self::winapi::GUID;

const NIF_ICON:  UINT = 0x2;
const NIF_TIP:   UINT = 0x4;
const NIF_INFO:  UINT = 0x10;
const NIIF_USER: UINT = 0x4;

const ERROR_CLASS_ALREADY_EXISTS: UINT = 1410;

#[repr(C)]
#[allow(non_snake_case)]
pub struct NOTIFYICONDATA {
    pub cbSize:           DWORD,
    pub hWnd:             HWND,
    pub uID:              UINT,
    pub uFlags:           UINT,
    pub uCallbackMessage: UINT,
    pub hIcon:            HICON,
    pub szTip:            [WCHAR; 128],
    pub dwState:          DWORD,
    pub dwStateMask:      DWORD,
    pub szInfo:           [WCHAR; 256],
    pub uVersion:         UINT,
    pub szInfoTitle:      [WCHAR; 64],
    pub dwInfoFlags:      DWORD,
    pub guidItem:         GUID,
    pub hBalloonIcon:     HICON,
}

#[link(name = "shell32")]
extern {
    pub fn Shell_NotifyIconW(mode: DWORD, iconData: *const NOTIFYICONDATA);
}

// We cannot implement the sync trait for arbitrary types (see E0117), so we use a struct to bypass it
pub struct NotifData {
    pub data: NOTIFYICONDATA,
}

impl NotifData {
    pub unsafe fn new() -> NotifData {
        let class_name = to_wstring(env!("CARGO_PKG_NAME"));

        let window = WNDCLASSW {
            style:         0,
            lpfnWndProc:   Some(window_proc),
            cbClsExtra:    0,
            cbWndExtra:    0,
            hInstance:     ptr::null_mut(),
            hIcon:         ptr::null_mut(),
            hCursor:       ptr::null_mut(),
            hbrBackground: ptr::null_mut(),
            lpszMenuName:  ptr::null(),
            lpszClassName: class_name.as_ptr(),
        };

        if user32::RegisterClassW(&window) == 0 {
            let error = kernel32::GetLastError() as UINT;

            // No reason to block the user from opening multiple program instances
            if error != ERROR_CLASS_ALREADY_EXISTS {
                panic!("unable to register notification window class: error {}", error);
            }
        }

        let h_wnd = user32::CreateWindowExW(
                        0,
                        class_name.as_ptr(),
                        class_name.as_ptr(),
                        0, 0, 0, 0, 0,
                        HWND_MESSAGE,
                        ptr::null_mut(),
                        ptr::null_mut(),
                        ptr::null_mut());

        if h_wnd.is_null() {
            panic!("unable to create notification window: error {}", kernel32::GetLastError());
        }

        let notif_data = NOTIFYICONDATA {
            cbSize:           mem::size_of::<NOTIFYICONDATA>() as DWORD,
            hWnd:             h_wnd,
            uID:              1,
            uFlags:           NIF_ICON | NIF_INFO | NIF_TIP,
            uCallbackMessage: WM_USER + 200,
            hIcon:            ptr::null_mut(),
            szTip:            [0; 128],
            dwState:          0,
            dwStateMask:      0,
            szInfo:           [0; 256],
            uVersion:         4,
            szInfoTitle:      [0; 64],
            dwInfoFlags:      NIIF_USER,
            guidItem:         winapi::GUID { Data1: 0, Data2: 0, Data3: 0, Data4: [0; 8] },
            hBalloonIcon:     0 as winapi::HICON,
        };

        NotifData {
            data: notif_data,
        }
    }
}

unsafe impl Send for NotifData {}
unsafe impl Sync for NotifData {}

pub fn to_wstring(str : &str) -> Vec<u16> {
    OsStr::new(str).encode_wide().chain(Some(0).into_iter()).collect()
}

unsafe extern "system" fn window_proc(h_wnd: HWND, msg: UINT, w_param: WPARAM, l_param: LPARAM) -> LRESULT
{
    if msg == WM_DESTROY {
        user32::PostQuitMessage(0);
    }

    user32::DefWindowProcW(h_wnd, msg, w_param, l_param)
}