mod config;
mod error;
mod feed;
mod path;

use crate::feed::stats::ListenerStats;
use crate::feed::{FeedData, FeedDisplay, FeedInfo};
use chrono::{Timelike, Utc};
use clap::{clap_app, ArgMatches};
use config::Config;
use error::Error;
use notify_rust::Notification;
use smallvec::SmallVec;
use std::time::Duration;

fn main() {
    let args = clap_app!(bcnotif =>
        (version: env!("CARGO_PKG_VERSION"))
        (author: env!("CARGO_PKG_AUTHORS"))
        (@arg DONT_SAVE_DATA: --nosave "Don't save feed data")
        (@arg RELOAD_CONFIG: -r --reloadconfig "Reload the configuration file on every update")
    )
    .get_matches();

    match run(args) {
        Ok(_) => (),
        Err(err) => {
            display_error(err);
            std::process::exit(1);
        }
    }
}

fn run(args: clap::ArgMatches) -> Result<(), Error> {
    let mut config = Config::load()?;

    let mut feed_data = {
        let path = FeedData::default_path()?;

        if path.exists() {
            FeedData::load(path)?
        } else {
            FeedData::new(path)
        }
    };

    loop {
        if args.is_present("RELOAD_CONFIG") {
            config = Config::load()?;
        }

        match run_update(&mut feed_data, &args, &config) {
            Ok(_) => (),
            Err(err) => display_error(err),
        }

        std::thread::sleep(Duration::from_secs((config.misc.update_time * 60.0) as u64));
    }
}

fn display_error<E>(err: E)
where
    E: Into<failure::Error>,
{
    let err = err.into();

    eprintln!("error: {}", err);

    for cause in err.iter_chain().skip(1) {
        eprintln!("  cause: {}", cause);
    }

    let backtrace = err.backtrace().to_string();

    if !backtrace.is_empty() {
        eprintln!("{}", backtrace);
    }

    Notification::new()
        .summary(concat!(env!("CARGO_PKG_NAME"), " error"))
        .body(&err.to_string())
        .show()
        .ok();
}

fn run_update(feed_data: &mut FeedData, args: &ArgMatches, config: &Config) -> Result<(), Error> {
    let feed_info = {
        let mut feeds = FeedInfo::scrape_from_config(config)?;
        filter_feeds(config, &mut feeds);
        feeds
    };

    let hour = Utc::now().hour() as usize;
    let mut displayed = SmallVec::<[FeedDisplay; 3]>::new();

    for info in feed_info {
        if info.listeners < config.misc.minimum_listeners {
            continue;
        }

        let stats = feed_data
            .stats
            .entry(info.id)
            .or_insert_with(ListenerStats::new);

        stats.update(hour, &info, config);

        if !feed::should_be_displayed(&info, &stats, config) {
            continue;
        }

        if displayed.len() > config.misc.max_feeds as usize {
            continue;
        }

        displayed.push(FeedDisplay::from(info, stats));
    }

    sort_feeds(&mut displayed, config);
    show_feeds(&displayed)?;

    if !args.is_present("DONT_SAVE_DATA") {
        feed_data.save()?;
    }

    Ok(())
}

fn filter_feeds(config: &Config, feeds: &mut Vec<FeedInfo>) {
    if !config.whitelist.is_empty() {
        feeds.retain(|feed| {
            config
                .whitelist
                .iter()
                .any(|entry| entry.matches_feed(feed))
        });
    }

    if !config.blacklist.is_empty() {
        feeds.retain(|feed| {
            config
                .blacklist
                .iter()
                .any(|entry| !entry.matches_feed(feed))
        });
    }
}

fn sort_feeds(feeds: &mut [FeedDisplay], config: &Config) {
    use config::{SortOrder, SortType};

    feeds.sort_unstable_by(|x, y| {
        let (x, y) = match config.sorting.sort_order {
            SortOrder::Ascending => (x, y),
            SortOrder::Descending => (y, x),
        };

        match config.sorting.sort_type {
            SortType::Listeners => x.info.listeners.cmp(&y.info.listeners),
            SortType::Jump => {
                let x_jump = x.jump as i32;
                let y_jump = y.jump as i32;

                x_jump.cmp(&y_jump)
            }
        }
    });
}

fn show_feeds(feeds: &[FeedDisplay]) -> Result<(), Error> {
    let total_feeds = feeds.len() as u32;

    for (i, feed) in feeds.iter().enumerate() {
        feed.show_notif(1 + i as u32, total_feeds)?;
    }

    Ok(())
}
