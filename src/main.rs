#![windows_subsystem = "windows"]

#[cfg(windows)] extern crate winrt;
#[macro_use]    extern crate error_chain;
extern crate chrono;

#[macro_use] mod util;
mod config;
mod error;
mod feed;
mod math;
mod notify;

use config::Config;
use chrono::{Utc, Timelike};
use feed::Feed;
use feed::statistics::{AverageData, ListenerStats};
use std::time::Duration;
use std::path::PathBuf;

error_chain! {
    links {
        Config(config::Error, config::ErrorKind);
        Feed(feed::Error, feed::ErrorKind);
        Statistics(feed::statistics::Error, feed::statistics::ErrorKind);
        Notify(notify::Error, notify::ErrorKind);
    }

    foreign_links {
        Io(std::io::Error);
    }
}

fn main() {
    #[cfg(windows)]
    let rt = winrt::RuntimeContext::init();

    match start() {
        Ok(_) => (),
        Err(err) => eprintln!("fatal error: {:?}", err),
    }

    #[cfg(windows)]
    rt.uninit();
}

fn start() -> Result<()> {
    let exe_dir = get_exe_directory()?;
    let config_path = exe_dir.clone().join("config.yaml");

    let mut averages = AverageData::new(exe_dir.join("averages.csv"));

    if averages.path.exists() {
        averages.load()?;
    }

    loop {
        let config = Config::from_file(&config_path)?;

        match perform_update(&mut averages, &config) {
            Ok(_) => (),
            Err(err) => error::display(&err),
        }

        std::thread::sleep(Duration::from_secs((config.misc.update_time * 60.0) as u64));
    }
}

fn perform_update(averages: &mut AverageData, config: &Config) -> Result<()> {
    let hour = Utc::now().hour();
    let mut display_feeds = Vec::new();

    for feed in Feed::download_and_scrape(&config)? {
        if feed.listeners < config.misc.minimum_listeners {
            continue
        }

        let stats = update_feed_stats(hour, &feed, &config, averages);

        if cfg!(feature = "print-feed-data") {
            print_info(&feed, &stats);
        }

        if stats.has_spiked || feed.alert.is_some() {
            display_feeds.push((feed, stats.clone()));
        }
    }

    show_feeds(display_feeds, &config)?;

    averages.save()?;
    Ok(())
}

fn update_feed_stats<'a>(hour: u32, feed: &Feed, config: &Config, averages: &'a mut AverageData)
    -> &'a ListenerStats {

    let stats = averages.data
        .entry(feed.id)
        .or_insert(ListenerStats::new());

    stats.update(hour as usize, feed, &config);
    stats
}

fn sort_feeds(feeds: &mut Vec<(Feed, ListenerStats)>, config: &Config) {
    use config::SortOrder;

    feeds.sort_unstable_by(|&(ref x, _), &(ref y, _)| {
        match config.misc.sort_order {
            SortOrder::Ascending  => x.listeners.cmp(&y.listeners),
            SortOrder::Descending => y.listeners.cmp(&x.listeners),
        }
    });
}

fn show_feeds(mut feeds: Vec<(Feed, ListenerStats)>, config: &Config) -> Result<()> {
    sort_feeds(&mut feeds, &config);

    let total = feeds.len() as i32;

    for (i, (feed, stats)) in feeds.into_iter().enumerate() {
        notify::create_update(1 + i as i32, total, &feed, &stats)?;
    }

    Ok(())
}

fn print_info(feed: &Feed, stats: &ListenerStats) {
    println!("[{}] {}", feed.id, feed.name);
    println!("\tlisteners    | {}", feed.listeners);

    println!("\taverage lis. | cur: {} last: {} samples: {:?}",
        stats.average.current,
        stats.average.last,
        stats.average.data);

    println!("\tunskewed avg | {:?}", stats.unskewed_average);
    println!("\thas spiked   | {}", stats.has_spiked);
    println!("\ttimes spiked | {}", stats.spike_count);
}

fn get_exe_directory() -> Result<PathBuf> {
    let mut path = std::env::current_exe()?;
    path.pop();
    Ok(path)
}