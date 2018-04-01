use feed::Feed;
use statistics::ListenerStats;
use std::borrow::Cow;

#[derive(Fail, Debug)]
pub enum NotifyError {
    #[cfg(any(unix, macos))]
    #[fail(display = "failed to create notification")]
    CreationFailed,

    #[cfg(windows)]
    #[fail(display = "{:?}", _0)]
    WinRT(::winrt::Error),
}

pub enum Icon {
    Update,
    Error,
}

#[cfg(any(unix, macos))]
mod unix {
    extern crate notify_rust;

    use self::notify_rust::Notification;
    use super::*;

    impl Icon {
        fn get_name(&self) -> &str {
            match *self {
                Icon::Update => "emblem-sound",
                Icon::Error => "dialog-error",
            }
        }
    }

    pub fn create(icon: &Icon, title: &str, body: &str) -> Result<(), NotifyError> {
        Notification::new()
            .summary(title)
            .body(body)
            .icon(icon.get_name())
            .show()
            .map_err(|_| NotifyError::CreationFailed)?;

        Ok(())
    }
}

#[cfg(windows)]
mod windows {
    use winrt::FastHString;
    use winrt::windows::data::xml::dom::*;
    use winrt::windows::ui::notifications::*;
    use super::{Icon, NotifyError};

    // https://stackoverflow.com/a/46817674
    //
    // The Toast Notification Manager needs a valid app ID for any notifications to actually display,
    // so we'll use one that is already defined since it is not worth the effort to create one ourselves.
    const APP_ID: &str = "{1AC14E77-02E7-4E5D-B744-2EB1AE5198B7}\\WindowsPowerShell\\v1.0\\powershell.exe";

    // Having this inner function will allow us to easily map all returned errors into the appropriate
    // "wrapper" error type with the create() function
    fn inner_create(title: &str, body: &str) -> Result<(), ::winrt::Error> {
        unsafe {
            let toast_xml =
                ToastNotificationManager::get_template_content(ToastTemplateType::ToastText02)?;

            let toast_text_elements =
                toast_xml.get_elements_by_tag_name(&FastHString::new("text"))?;

            let add_text = |i, string| {
                let node = &*toast_xml
                    .create_text_node(&FastHString::new(string))?
                    .query_interface::<IXmlNode>()
                    .unwrap();

                toast_text_elements.item(i)?.append_child(node)
            };

            add_text(0, title)?;
            add_text(1, body)?;

            let toast = ToastNotification::create_toast_notification(&*toast_xml)?;

            ToastNotificationManager::create_toast_notifier_with_id(&FastHString::new(APP_ID))?
                .show(&*toast)?;
        }

        Ok(())
    }

    pub fn create(_: &Icon, title: &str, body: &str) -> Result<(), NotifyError> {
        inner_create(title, body).map_err(NotifyError::WinRT)
    }
}

#[cfg(any(unix, macos))]
use self::unix::create;

#[cfg(windows)]
use self::windows::create;

pub fn create_update(
    index: i32,
    max_index: i32,
    feed: &Feed,
    feed_stats: &ListenerStats,
) -> Result<(), NotifyError> {
    let title = format!(
        "{} - Broadcastify Update ({} of {})",
        feed.state.abbrev, index, max_index
    );

    let alert = match feed.alert {
        Some(ref alert) => Cow::Owned(format!("\nAlert: {}", alert)),
        None => Cow::Borrowed(""),
    };

    let body = format!(
        "Name: {}\nListeners: {} (^{}){}\nLink: http://broadcastify.com/listen/feed/{}",
        feed.name,
        feed.listeners,
        feed_stats.get_jump(feed.listeners) as i32,
        &alert,
        feed.id
    );

    create(&Icon::Update, &title, &body)
}

pub fn create_error(body: &str) -> Result<(), NotifyError> {
    create(&Icon::Error, "Broadcastify Update Error", body)
}
