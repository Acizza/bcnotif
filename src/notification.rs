extern crate notify_rust;

use self::notify_rust::{Notification, Error};
use feed::Feed;

fn create(icon: &str, title: &str, body: &str) -> Result<(), Error> {
    Notification::new()
        .summary(title)
        .body(body)
        .icon(icon)
        .show()
        .map(|_| ())
}

pub fn create_update(feed_idx: i32, max_feed_idx: i32, feed: &Feed, feed_delta: i32) ->
    Result<(), Error> {
        
    let alert = match feed.alert {
        Some(ref alert) => format!("\nAlert: {}", alert),
        None            => String::new(),
    };

    create(
        "emblem-sound",
        &format!("Broadcastify Update ({} of {})", feed_idx, max_feed_idx),
        &format!("Name: {}\nListeners: {} (^{}){}\nLink: http://broadcastify.com/listen/feed/{}",
            feed.name,
            feed.listeners,
            feed_delta,
            &alert,
            feed.id))
}

pub fn create_error(body: &str) -> Result<(), Error> {
    create(
        "dialog-error",
        "Broadcastify Update Error",
        body)
}