use failure;
use notify;

#[derive(Fail, Debug)]
pub enum Error {
    #[fail(display = "io error")]
    Io(#[cause] ::std::io::Error),

    #[fail(display = "config error")]
    Config(#[cause] ConfigError),

    #[fail(display = "feed error")]
    Feed(#[cause] FeedError),

    #[fail(display = "statistics error")]
    Statistics(#[cause] StatisticsError),
}

#[derive(Fail, Debug)]
pub enum FeedError {
    #[fail(display = "notification error")]
    NotifyError(#[cause] NotifyError),

    #[fail(display = "HTTP error")]
    Reqwest(#[cause] ::reqwest::Error),

    #[fail(display = "failed to parse top feeds")]
    ParseTopFeeds(#[cause] ScrapeError),

    #[fail(display = "failed to parse state ({}) feeds", _1)]
    ParseStateFeeds(#[cause] ScrapeError, String),
}

type ElementName = &'static str;

#[derive(Fail, Debug)]
pub enum ScrapeError {
    #[fail(display = "unable to find element that contains {} information", _0)]
    NoElement(ElementName),

    #[fail(display = "unable to parse {} information", _1)]
    FailedIntParse(#[cause] ::std::num::ParseIntError, ElementName),

    #[fail(display = "no feeds found")]
    NoneFound,
}

#[derive(Fail, Debug)]
pub enum StatisticsError {
    #[fail(display = "CSV error")]
    CSV(#[cause] ::csv::Error),

    #[fail(display = "io error")]
    Io(#[cause] ::std::io::Error),

    #[fail(display = "failed to parse integer")]
    ParseIntError(#[cause] ::std::num::ParseIntError),

    #[fail(display = "failed to parse float")]
    ParseFloatError(#[cause] ::std::num::ParseFloatError),

    #[fail(display = "CSV file contains record with too few rows")]
    TooFewRows,
}

#[derive(Fail, Debug)]
pub enum NotifyError {
    #[cfg(not(windows))]
    #[fail(display = "failed to create notification")]
    CreationFailed,

    #[cfg(windows)]
    #[fail(display = "WinRT error")]
    WinRT(::winrt::Error),

    #[cfg(windows)]
    #[fail(display = "notification element is null: {}", _0)]
    NullElement(String),
}

#[derive(Fail, Debug)]
pub enum ConfigError {
    #[fail(display = "io error")]
    Io(#[cause] ::std::io::Error),

    #[fail(display = "error parsing YAML")]
    YAMLScan(#[cause] ::yaml_rust::ScanError),
}

fn build_err_msg(err: &failure::Error) -> String {
    let mut msg = format!("error: {}\n", err.cause());

    for cause in err.causes().skip(1) {
        msg.push_str(&format!("caused by: {}\n", cause));
    }

    msg
}

fn print_with_backtrace(msg: &str, err: &failure::Error) {
    eprintln!("{}", msg);
    eprintln!("{}", err.backtrace());
}

/// Displays the provided error with a notification and by writing it to the terminal
pub fn display(err: &failure::Error) {
    let msg = build_err_msg(err);
    print_with_backtrace(&msg, err);

    match notify::create_error(&msg) {
        Ok(_) => (),
        Err(notif_err) => {
            eprintln!("failed to create error notification:");

            let notif_err = notif_err.into();
            let notif_msg = build_err_msg(&notif_err);
            print_with_backtrace(&notif_msg, &notif_err);
        }
    }
}
