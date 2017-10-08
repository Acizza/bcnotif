use feed::Feed;
use feed::statistics::ListenerStats;

error_chain! {
    errors {
        CreationFailed {
            display("failed to create notification")
        }
    }
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
                Icon::Error  => "dialog-error",
            }
        }
    }

    pub fn create(icon: Icon, title: &str, body: &str) -> Result<()> {
        Notification::new()
            .summary(title)
            .body(body)
            .icon(icon.get_name())
            .show()
            .chain_err(|| ErrorKind::CreationFailed)?;

        Ok(())
    }
}

#[cfg(any(unix, macos))]
use self::unix::create;

pub fn create_update(index: i32, max_index: i32, feed: &Feed,
    feed_stats: &ListenerStats) -> Result<()> {

    let title = format!(
        "{} - Broadcastify Update ({} of {})",
        feed.get_state_abbrev().unwrap_or("UNK"),
        index,
        max_index);

    let alert = match feed.alert {
        Some(ref alert) => format!("\nAlert: {}", alert),
        None            => String::new(),
    };

    let body = format!(
        "Name: {}\nListeners: {} (^{}){}\nLink: http://broadcastify.com/listen/feed/{}",
        feed.name,
        feed.listeners,
        feed_stats.get_average_delta(feed.listeners) as i32,
        &alert,
        feed.id);

    create(Icon::Update, &title, &body)
}

pub fn create_error(body: &str) -> Result<()> {
    create(Icon::Error, "Broadcastify Update Error", body)
}