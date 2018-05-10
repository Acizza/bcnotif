#![windows_subsystem = "windows"]

#[macro_use]
extern crate failure;

#[macro_use]
extern crate lazy_static;

extern crate chrono;
extern crate csv;
extern crate directories;
extern crate reqwest;
extern crate select;
extern crate yaml_rust;

#[cfg(windows)]
extern crate winrt;

mod config;
mod error;
mod feed;
mod notify;
mod path;

use chrono::{Timelike, Utc};
use config::Config;
use error::Error;
use feed::Feed;
use feed::statistics::{AverageData, ListenerStats};
use std::time::Duration;

fn main() {
    #[cfg(windows)]
    let rt = winrt::RuntimeContext::init();

    match run() {
        Ok(_) => (),
        Err(err) => {
            eprintln!("error during init:");
            error::display(&err.into());
        }
    }

    #[cfg(windows)]
    rt.uninit();
}

fn run() -> Result<(), Error> {
    let mut averages = AverageData::load().map_err(Error::Statistics)?;

    loop {
        let config = Config::load().map_err(Error::Config)?;

        match perform_update(&mut averages, &config) {
            Ok(_) => (),
            Err(err) => error::display(&err.into()),
        }

        std::thread::sleep(Duration::from_secs((config.misc.update_time * 60.0) as u64));
    }
}

fn perform_update(averages: &mut AverageData, config: &Config) -> Result<(), Error> {
    let hour = Utc::now().hour() as usize;
    let mut display_feeds = Vec::new();

    let feeds = feed::scrape_all(config)
        .map_err(Error::Feed)?
        .into_iter()
        .filter(|feed| feed.listeners >= config.misc.minimum_listeners);

    for feed in feeds {
        let stats = averages.get_feed_stats(&feed);
        stats.update(hour, &feed, config);

        if cfg!(feature = "print-feed-data") {
            print_info(&feed, stats);
        }

        // TODO: Move to statistics module (along with some other misc functionality)
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

    averages.save().map_err(Error::Statistics)?;
    Ok(())
}

fn sort_feeds(feeds: &mut Vec<(Feed, ListenerStats)>, config: &Config) {
    use config::{SortOrder, SortType};

    feeds.sort_unstable_by(|x, y| {
        let (x, y) = match config.sorting.sort_order {
            SortOrder::Ascending => (x, y),
            SortOrder::Descending => (y, x),
        };

        let &(ref x_feed, ref x_stats) = x;
        let &(ref y_feed, ref y_stats) = y;

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

    for (i, &(ref feed, ref stats)) in feeds.iter().enumerate() {
        feed.show_notification(stats, 1 + i as u32, total_feeds)
            .map_err(Error::Feed)?;
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
