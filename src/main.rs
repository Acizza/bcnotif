#![windows_subsystem = "windows"]

#[macro_use]
extern crate failure;

#[macro_use]
extern crate lazy_static;

extern crate chrono;
extern crate csv;
extern crate reqwest;
extern crate select;
extern crate yaml_rust;

#[cfg(windows)]
extern crate winrt;

#[macro_use]
mod util;

mod config;
mod error;
mod feed;
mod notify;

use config::Config;
use chrono::{Timelike, Utc};
use error::Error;
use feed::{Feed, statistics::{AverageData, ListenerStats}};
use std::time::Duration;
use std::path::PathBuf;

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
    let exe_dir = get_exe_directory().map_err(Error::Io)?;
    let config_path = exe_dir.clone().join("config.yaml");

    let mut averages = AverageData::new(exe_dir.join("averages.csv"));

    if averages.path.exists() {
        averages.load().map_err(Error::Statistics)?;
    }

    loop {
        let config = if !config_path.exists() {
            Config::default()
        } else {
            Config::from_file(&config_path).map_err(Error::Config)?
        };

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

    let feeds = feed::scrape_all(config).map_err(Error::Feed)?;

    for feed in feeds {
        if feed.listeners < config.misc.minimum_listeners {
            continue;
        }

        let stats = averages.update_feed_stats(&feed, config, hour);

        if cfg!(feature = "print-feed-data") {
            print_info(&feed, stats);
        }

        let can_show = stats.has_spiked || feed.alert.is_some();

        if can_show && (display_feeds.len() as u32) < config.misc.max_feeds {
            display_feeds.push((feed, stats.clone()));
        }
    }

    show_feeds(display_feeds, config)?;

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

        let x_feed = &x.0;
        let x_stats = &x.1;

        let y_feed = &y.0;
        let y_stats = &y.1;

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

fn show_feeds(mut feeds: Vec<(Feed, ListenerStats)>, config: &Config) -> Result<(), Error> {
    sort_feeds(&mut feeds, config);

    let total_feeds = feeds.len() as u32;

    for (i, (feed, stats)) in feeds.into_iter().enumerate() {
        feed.show_notification(&stats, 1 + i as u32, total_feeds)
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

fn get_exe_directory() -> std::io::Result<PathBuf> {
    let mut path = std::env::current_exe()?;
    path.pop();
    Ok(path)
}
