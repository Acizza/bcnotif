mod config;
mod error;
mod feed;
mod path;

use crate::feed::statistics::{AverageData, ListenerStats};
use crate::feed::Feed;
use chrono::{Timelike, Utc};
use clap::clap_app;
use config::Config;
use error::Error;
use notify_rust::Notification;
use std::time::Duration;

fn main() {
    let args = clap_app!(bcnotif =>
        (version: env!("CARGO_PKG_VERSION"))
        (author: env!("CARGO_PKG_AUTHORS"))
        (@arg DONT_SAVE_DATA: --nosave "Don't save feed data")
        (@arg RELOAD_CONFIG: -r --reloadconfig "Reload the configuration file on every update")
        (@arg PRINT_FEED_DATA: --printdata "Print detailed feed data / statistics on every update")
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
    let mut averages = AverageData::load()?;
    let mut config = Config::load()?;

    loop {
        if args.is_present("RELOAD_CONFIG") {
            config = Config::load()?;
        }

        match perform_update(&mut averages, &args, &config) {
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
        .summary(&format!("Error in {}", env!("CARGO_PKG_NAME")))
        .body(&err.to_string())
        .show()
        .ok();
}

fn perform_update(
    averages: &mut AverageData,
    args: &clap::ArgMatches,
    config: &Config,
) -> Result<(), Error> {
    let hour = Utc::now().hour() as usize;
    let mut display_feeds = Vec::new();

    let feeds = feed::scrape_all(config)?
        .into_iter()
        .filter(|feed| feed.listeners >= config.misc.minimum_listeners);

    for feed in feeds {
        let stats = averages.get_feed_stats(&feed);
        stats.update(hour, &feed, config);

        if args.is_present("PRINT_FEED_DATA") {
            print_info(&feed, stats);
        }

        if let Some(max_times) = config.misc.max_times_to_show_feed {
            if stats.spike_count > max_times {
                continue;
            }
        }

        let show_for_alert = feed.alert.is_some() && config.misc.show_alert_feeds;
        let can_show = stats.has_spiked || show_for_alert;

        if can_show && (display_feeds.len() as u32) < config.misc.max_feeds {
            display_feeds.push((feed, stats.clone()));
        }
    }

    sort_feeds(&mut display_feeds, config);
    show_feeds(&display_feeds)?;

    if !args.is_present("DONT_SAVE_DATA") {
        averages.save()?;
    }

    Ok(())
}

fn sort_feeds(feeds: &mut Vec<(Feed, ListenerStats)>, config: &Config) {
    use config::{SortOrder, SortType};

    feeds.sort_unstable_by(|x, y| {
        let (x, y) = match config.sorting.sort_order {
            SortOrder::Ascending => (x, y),
            SortOrder::Descending => (y, x),
        };

        let (x_feed, x_stats) = x;
        let (y_feed, y_stats) = y;

        match config.sorting.sort_type {
            SortType::Listeners => x_feed.listeners.cmp(&y_feed.listeners),
            SortType::Jump => {
                let x_jump = x_stats.get_jump(x_feed.listeners) as i32;
                let y_jump = y_stats.get_jump(y_feed.listeners) as i32;

                x_jump.cmp(&y_jump)
            }
        }
    });
}

fn show_feeds(feeds: &[(Feed, ListenerStats)]) -> Result<(), Error> {
    let total_feeds = feeds.len() as u32;

    for (i, (feed, stats)) in feeds.iter().enumerate() {
        feed.show_notification(stats, 1 + i as u32, total_feeds)?;
    }

    Ok(())
}

fn print_info(feed: &Feed, stats: &ListenerStats) {
    println!("[{}] {}", feed.id, feed.name);
    println!("\tlisteners    | {}", feed.listeners);

    println!(
        "\taverage lis. | cur: {} last: {} samples: {:?}",
        stats.average.current, stats.average.last, stats.average.data
    );

    println!("\tunskewed avg | {:?}", stats.unskewed_average);
    println!("\thas spiked   | {}", stats.has_spiked);
    println!("\ttimes spiked | {}", stats.spike_count);
}
