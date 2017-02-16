use std::error::Error;
use feed::Feed;

#[derive(Debug)]
pub enum Icon {
    Update,
    Error,
}

#[cfg(unix)]
mod unix {
    extern crate notify_rust;
    use std::error::Error;
    use notification::Icon;
    use self::notify_rust::Notification;

    pub fn create(icon: Icon, title: &str, body: &str) -> Result<(), Box<Error>> {
        let icon = match icon {
            Icon::Update => "emblem-sound",
            Icon::Error  => "dialog-error",
        };

        Notification::new()
            .summary(title)
            .body(body)
            .icon(icon)
            .show()
            .map(|_| ())
            .map_err(|e| e.into())
    }
}

#[cfg(windows)]
mod windows {
    extern crate winapi;
    extern crate user32;
    extern crate kernel32;

    use std::cmp;
    use std::error::Error;
    use std::ffi::OsStr;
    use std::mem;
    use std::os::windows::ffi::OsStrExt;
    use std::ptr;
    use std::sync::Mutex;
    use self::winapi::winuser::{WNDCLASSW, HWND_MESSAGE, WM_USER, WM_DESTROY, IDI_INFORMATION, IDI_ERROR};
    use self::winapi::minwindef::{DWORD, UINT, LRESULT, LPARAM, HINSTANCE, WPARAM};
    use self::winapi::windef::{HWND, HICON};
    use self::winapi::winnt::WCHAR;
    use self::winapi::GUID;
    use notification::Icon;

    const NIF_ICON: UINT   = 0x2;
    const NIF_TIP: UINT    = 0x4;
    const NIF_INFO: UINT   = 0x10;
    const NIM_ADD: UINT    = 0;
    const NIM_DELETE: UINT = 2;
    const NIIF_USER: UINT = 0x4;

    const ERROR_CLASS_ALREADY_EXISTS: UINT = 1410;

    #[repr(C)]
    #[allow(non_snake_case)]
    struct NOTIFYICONDATA {
        cbSize:           DWORD,
        hWnd:             HWND,
        uID:              UINT,
        uFlags:           UINT,
        uCallbackMessage: UINT,
        hIcon:            HICON,
        szTip:            [WCHAR; 128],
        dwState:          DWORD,
        dwStateMask:      DWORD,
        szInfo:           [WCHAR; 256],
        uVersion:         UINT,
        szInfoTitle:      [WCHAR; 64],
        dwInfoFlags:      DWORD,
        guidItem:         GUID,
        hBalloonIcon:     HICON,
    }

    #[link(name = "shell32")]
    extern {
        fn Shell_NotifyIconW(mode: DWORD, iconData: *const NOTIFYICONDATA);
    }

    // We cannot implement the sync trait for arbitrary types (see E0117), so we use a struct to bypass it
    struct NotifData {
        data: NOTIFYICONDATA,
    }

    impl NotifData {
        unsafe fn new() -> NotifData {
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

    pub fn create(icon: Icon, title: &str, body: &str) -> Result<(), Box<Error>> {
        lazy_static! {
            static ref NOTIF_DATA: Mutex<NotifData> = unsafe {
                Mutex::new(NotifData::new())
            };
        }

        let mut notif = NOTIF_DATA.lock().unwrap();
        let notif = &mut notif.data;

        let icon = match icon {
            Icon::Update => IDI_INFORMATION,
            Icon::Error  => IDI_ERROR,
        };

        unsafe {
            let new_icon = user32::LoadIconW(0 as HINSTANCE, icon);

            if !new_icon.is_null() {
                notif.hIcon = new_icon;
            }

            let title = to_wstring(title);
            ptr::copy(title.as_ptr(), notif.szInfoTitle.as_mut_ptr(), cmp::min(title.len(), notif.szInfoTitle.len()));

            let body = to_wstring(body);
            ptr::copy(body.as_ptr(), notif.szInfo.as_mut_ptr(), cmp::min(body.len(), notif.szInfo.len()));

            Shell_NotifyIconW(NIM_ADD, notif);
            Shell_NotifyIconW(NIM_DELETE, notif);
        }
        
        Ok(())
    }

    fn to_wstring(str : &str) -> Vec<u16> {
        OsStr::new(str).encode_wide().chain(Some(0).into_iter()).collect()
    }

    unsafe extern "system" fn window_proc(h_wnd: HWND, msg: UINT, w_param: WPARAM, l_param: LPARAM) -> LRESULT
    {
        if msg == WM_DESTROY {
            user32::PostQuitMessage(0);
        }

        user32::DefWindowProcW(h_wnd, msg, w_param, l_param)
    }
}

#[cfg(unix)]
use self::unix::create;

#[cfg(windows)]
use self::windows::create;

pub fn create_update(feed_idx: i32, max_feed_idx: i32, feed: &Feed, feed_delta: i32) ->
    Result<(), Box<Error>> {
        
    let alert = match feed.alert {
        Some(ref alert) => format!("\nAlert: {}", alert),
        None            => String::new(),
    };

    create(
        Icon::Update,
        &format!("Broadcastify Update ({} of {})", feed_idx, max_feed_idx),
        &format!("Name: {}\nListeners: {} (^{}){}\nLink: http://broadcastify.com/listen/feed/{}",
            feed.name,
            feed.listeners,
            feed_delta,
            &alert,
            feed.id))
}

pub fn create_error(body: &str) -> Result<(), Box<Error>> {
    create(
        Icon::Error,
        "Broadcastify Update Error",
        body)
}