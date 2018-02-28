use failure::Error;
use feed::Feed;
use feed::statistics::ListenerStats;
use std::borrow::Cow;

pub enum Icon {
    Update,
    Error,
}

#[cfg(any(unix, macos))]
mod unix {
    extern crate notify_rust;

    use self::notify_rust::Notification;
    use super::*;

    #[derive(Fail, Debug)]
    #[fail(display = "failed to create notification")]
    pub struct CreationFailedError;

    impl Icon {
        fn get_name(&self) -> &str {
            match *self {
                Icon::Update => "emblem-sound",
                Icon::Error => "dialog-error",
            }
        }
    }

    pub fn create(icon: &Icon, title: &str, body: &str) -> Result<(), CreationFailedError> {
        Notification::new()
            .summary(title)
            .body(body)
            .icon(icon.get_name())
            .show()
            .map_err(|_| CreationFailedError)?;

        Ok(())
    }
}

#[cfg(windows)]
mod windows {
    use failure::Error;
    use winrt::FastHString;
    use winrt::windows::data::xml::dom::*;
    use winrt::windows::ui::notifications::*;
    use super::Icon;

    // The purpose of having an inner create function is so that we only have to specify the error
    // type once if creation fails
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
            let id = env!("CARGO_PKG_NAME");

            ToastNotificationManager::create_toast_notifier_with_id(&FastHString::new(id))?
                .show(&*toast)?;
        }

        Ok(())
    }

    pub fn create(_: &Icon, title: &str, body: &str) -> Result<(), Error> {
        inner_create(title, body).map_err(|err| format_err!("{:?}", err))?;

        Ok(())
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
) -> Result<(), Error> {
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

    create(&Icon::Update, &title, &body)?;
    Ok(())
}

pub fn create_error(body: &str) -> Result<(), Error> {
    create(&Icon::Error, "Broadcastify Update Error", body)?;
    Ok(())
}
