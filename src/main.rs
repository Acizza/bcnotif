mod config;
mod err;
mod feed;
mod path;

use crate::feed::stats::{ListenerStatMap, ListenerStats};
use crate::feed::{Feed, FeedDisplay};
use chrono::{Timelike, Utc};
use clap::clap_app;
use config::Config;
use err::Result;
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
            err::display_error(err);
            std::process::exit(1);
        }
    }
}

fn run(args: clap::ArgMatches) -> Result<()> {
    let mut config = Config::load()?;
    let mut listener_stats = ListenerStatMap::load_or_new()?;

    let reload_config = args.is_present("RELOAD_CONFIG");
    let save_data = !args.is_present("DONT_SAVE_DATA");

    loop {
        if reload_config {
            match Config::load() {
                Ok(new) => config = new,
                Err(err) => err::display_error(err),
            }
        }

        match run_update(&mut listener_stats, save_data, &config) {
            Ok(_) => (),
            Err(err) => err::display_error(err),
        }

        std::thread::sleep(Duration::from_secs((config.misc.update_time * 60.0) as u64));
    }
}

fn run_update(
    listener_stats: &mut ListenerStatMap,
    save_data: bool,
    config: &Config,
) -> Result<()> {
    let feed_info = {
        let mut feeds = Feed::scrape_all(config)?;
        filter_feeds(config, &mut feeds);
        feeds
    };

    let hour = Utc::now().hour() as usize;
    let mut displayed = SmallVec::<[FeedDisplay; 3]>::new();

    for info in feed_info {
        let stats = listener_stats
            .stats_mut()
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

    if save_data {
        listener_stats.save()?;
    }

    Ok(())
}

fn filter_feeds(config: &Config, feeds: &mut Vec<Feed>) {
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
            SortType::Listeners => x.feed.listeners.cmp(&y.feed.listeners),
            SortType::Jump => {
                let x_jump = x.jump as i32;
                let y_jump = y.jump as i32;

                x_jump.cmp(&y_jump)
            }
        }
    });
}

fn show_feeds(feeds: &[FeedDisplay]) -> Result<()> {
    let total_feeds = feeds.len() as u32;

    for (i, feed) in feeds.iter().enumerate() {
        feed.show_notif(1 + i as u32, total_feeds)?;
    }

    Ok(())
}
