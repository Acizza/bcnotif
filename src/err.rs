use crate::feed;
use notify_rust::Notification;
use snafu::{Backtrace, ErrorCompat, GenerateBacktrace, Snafu};
use std::io;
use std::path;
use std::sync::mpsc;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)))]
pub enum Error {
    #[snafu(display("io error: {}", source))]
    IO {
        source: io::Error,
        backtrace: Backtrace,
    },

    #[snafu(display("file io error at {}: {}", path.display(), source))]
    FileIO {
        path: path::PathBuf,
        source: io::Error,
        backtrace: Backtrace,
    },

    #[snafu(display("reqwest error: {}", source))]
    Reqwest {
        source: reqwest::Error,
        backtrace: Backtrace,
    },

    #[snafu(display("diesel error: {}", source))]
    Diesel {
        source: diesel::result::Error,
        backtrace: Backtrace,
    },

    #[snafu(display("diesel connection error: {}", source))]
    DieselConnection {
        source: diesel::result::ConnectionError,
        backtrace: Backtrace,
    },

    #[snafu(display("error decoding TOML {} file: {}", name, source))]
    TOMLDecode {
        name: &'static str,
        source: toml::de::Error,
        backtrace: Backtrace,
    },

    #[snafu(display("mpsc receive error: {}", source))]
    MPSCRecv {
        source: mpsc::RecvError,
        backtrace: Backtrace,
    },

    #[snafu(display("failed to register signal handler: {}", source))]
    Signal { source: nix::Error },

    #[snafu(display("failed to parse top feeds: {}", source))]
    ParseTopFeeds { source: ScrapeError },

    #[snafu(display("failed to parse feeds for {}: {}", location.abbrev(), source))]
    ParseLocationFeeds {
        location: feed::Location,
        source: ScrapeError,
    },

    #[snafu(display("failed to create notification: {}", source))]
    CreateNotif { source: notify_rust::Error },
}

impl Error {
    pub fn is_file_nonexistant(&self) -> bool {
        match self {
            Error::FileIO { source, .. } => source.kind() == io::ErrorKind::NotFound,
            _ => false,
        }
    }
}

impl From<io::Error> for Error {
    fn from(source: io::Error) -> Self {
        Self::IO {
            source,
            backtrace: Backtrace::generate(),
        }
    }
}

impl From<reqwest::Error> for Error {
    fn from(source: reqwest::Error) -> Self {
        Self::Reqwest {
            source,
            backtrace: Backtrace::generate(),
        }
    }
}

impl From<diesel::result::Error> for Error {
    fn from(source: diesel::result::Error) -> Self {
        Self::Diesel {
            source,
            backtrace: Backtrace::generate(),
        }
    }
}

impl From<diesel::result::ConnectionError> for Error {
    fn from(source: diesel::result::ConnectionError) -> Self {
        Self::DieselConnection {
            source,
            backtrace: Backtrace::generate(),
        }
    }
}

impl From<mpsc::RecvError> for Error {
    fn from(source: mpsc::RecvError) -> Self {
        Self::MPSCRecv {
            source,
            backtrace: Backtrace::generate(),
        }
    }
}

#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)))]
pub enum ScrapeError {
    #[snafu(display("no feeds found"))]
    NoFeeds,

    #[snafu(display("missing feed table"))]
    MissingFeedTable,

    #[snafu(display("unknown feed location: {}", id))]
    UnknownLocation { id: u32 },
}

pub fn display_error(err: Error) {
    let err_str = format!("{}", err);

    eprintln!("{}", err_str);

    if let Some(backtrace) = err.backtrace() {
        eprintln!("backtrace:\n{}", backtrace);
    }

    Notification::new()
        .summary(concat!(env!("CARGO_PKG_NAME"), " error"))
        .body(&err_str)
        .show()
        .ok();
}
