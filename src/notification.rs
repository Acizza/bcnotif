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
    use std::ptr;
    use std::sync::Mutex;
    use self::winapi::winuser::{IDI_INFORMATION, IDI_ERROR};
    use self::winapi::minwindef::{UINT, HINSTANCE};
    use notification::Icon;
    use util::windows::*;

    const NIM_ADD: UINT    = 0;
    const NIM_DELETE: UINT = 2;

    pub fn create(icon: Icon, title: &str, body: &str) -> Result<(), Box<Error>> {
        lazy_static! {
            static ref NOTIF_DATA: Mutex<NotifData> = unsafe {
                Mutex::new(NotifData::new())
            };
        }

        let mut notif = NOTIF_DATA.lock()?;
        let notif = &mut notif.data;

        let icon = match icon {
            Icon::Update => IDI_INFORMATION,
            Icon::Error  => IDI_ERROR,
        };

        unsafe {
            notif.hIcon = user32::LoadIconW(0 as HINSTANCE, icon);

            let title = to_wstring(title);
            ptr::copy(title.as_ptr(), notif.szInfoTitle.as_mut_ptr(), cmp::min(title.len(), notif.szInfoTitle.len()));

            let body = to_wstring(body);
            ptr::copy(body.as_ptr(), notif.szInfo.as_mut_ptr(), cmp::min(body.len(), notif.szInfo.len()));

            Shell_NotifyIconW(NIM_ADD, notif);
            Shell_NotifyIconW(NIM_DELETE, notif);
        }
        
        Ok(())
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