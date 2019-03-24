use failure::Fail;

macro_rules! impl_error_conversion {
    ($err_name:ident, $($from_ty:ty => $to_ty:ident,)+) => {
        $(
        impl From<$from_ty> for $err_name {
            fn from(f: $from_ty) -> $err_name {
                $err_name::$to_ty(f)
            }
        }
        )+
    };
}

#[derive(Fail, Debug)]
pub enum Error {
    #[fail(display = "config error")]
    Config(#[cause] ConfigError),

    #[fail(display = "feed error")]
    Feed(#[cause] FeedError),
}

impl_error_conversion!(Error,
    ConfigError => Config,
    FeedError => Feed,
);

#[derive(Fail, Debug)]
pub enum FeedError {
    #[fail(display = "io error")]
    Io(#[cause] ::std::io::Error),

    #[fail(display = "HTTP error")]
    Reqwest(#[cause] ::reqwest::Error),

    #[fail(display = "failed to parse top feeds")]
    ParseTopFeeds(#[cause] ScrapeError),

    #[fail(display = "failed to parse state ({}) feeds", _1)]
    ParseStateFeeds(#[cause] ScrapeError, u32),

    #[fail(display = "failed to create notification")]
    FailedToCreateNotification,

    #[fail(display = "malformed csv data")]
    MalformedCSV,
}

impl_error_conversion!(FeedError,
    std::io::Error => Io,
    reqwest::Error => Reqwest,
);

type ElementName = &'static str;

#[derive(Fail, Debug)]
pub enum ScrapeError {
    #[fail(display = "unable to parse {} information", _1)]
    FailedIntParse(#[cause] ::std::num::ParseIntError, ElementName),

    #[fail(display = "no feeds found")]
    NoneFound,

    #[fail(display = "search string not found: {}", _0)]
    SearchStringNotFound(String),

    #[fail(display = "feed table has an invalid number of columns")]
    InvalidNumberOfColumns,

    #[fail(display = "feed did not have location info")]
    NoLocationInfo,
}

#[derive(Fail, Debug)]
pub enum ConfigError {
    #[fail(display = "io error")]
    Io(#[cause] ::std::io::Error),

    #[fail(display = "error parsing YAML")]
    YAMLScan(#[cause] ::yaml_rust::ScanError),
}

impl_error_conversion!(ConfigError,
    ::std::io::Error => Io,
    ::yaml_rust::ScanError => YAMLScan,
);
