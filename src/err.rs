use notify_rust::Notification;
use snafu::{Backtrace, ErrorCompat, GenerateBacktrace, Snafu};
use std::io;
use std::num;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)))]
pub enum Error {
    #[snafu(display("io error: {}", source))]
    IO {
        source: io::Error,
        backtrace: Backtrace,
    },

    #[snafu(display("reqwest error: {}", source))]
    Reqwest {
        source: reqwest::Error,
        backtrace: Backtrace,
    },

    #[snafu(display("YAML error: {}", source))]
    YAMLScan {
        source: yaml_rust::ScanError,
        backtrace: Backtrace,
    },

    #[snafu(display("failed to parse top feeds: {}", source))]
    ParseTopFeeds { source: ScrapeError },

    #[snafu(display("failed to parse state feeds: {}", source))]
    ParseStateFeeds { source: ScrapeError },

    #[snafu(display("failed to create notification: {}", source))]
    CreateNotif { source: notify_rust::Error },

    #[snafu(display("malformed csv data"))]
    MalformedCSV,
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

impl From<yaml_rust::ScanError> for Error {
    fn from(source: yaml_rust::ScanError) -> Self {
        Self::YAMLScan {
            source,
            backtrace: Backtrace::generate(),
        }
    }
}

#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)))]
pub enum ScrapeError {
    #[snafu(display("unable to parse {} information", element))]
    FailedIntParse {
        source: num::ParseIntError,
        element: &'static str,
    },

    #[snafu(display("no feeds found"))]
    NoFeeds,

    #[snafu(display("search string not found: {}", string))]
    SearchStringNotFound { string: String },

    #[snafu(display("feed table has an invalid number of columns"))]
    InvalidNumberOfColumns,

    #[snafu(display("feed did not have location info"))]
    NoLocationInfo,
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
