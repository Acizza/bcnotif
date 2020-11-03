use anyhow::Error;
use notify_rust::Notification;
use std::io;

pub fn is_file_nonexistant(err: &Error) -> bool {
    matches!(err.downcast_ref::<io::Error>(), Some(err) if err.kind() == io::ErrorKind::NotFound)
}

pub fn error_notif(err: &Error) {
    Notification::new()
        .summary(concat!(env!("CARGO_PKG_NAME"), " error"))
        .body(&format!("{:?}", err))
        .show()
        .ok();
}
