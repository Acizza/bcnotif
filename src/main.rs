extern crate chrono;
#[macro_use] extern crate lazy_static;

#[macro_use] mod util;
mod config;
mod feed;
mod notification;

use std::thread;
use std::time::Duration;
use std::collections::HashMap;
use feed::listeners::{self, AverageMap, ListenerData};
use config::Config;
use util::error::{self, DetailedError};
use self::chrono::{UTC, Timelike};

fn sort_feeds(config: &Config, feeds: &mut Vec<feed::Feed>) {
    use config::SortOrder::*;

    feeds.sort_by(|x, y| {
        match config.misc.sort_order {
            Ascending  => x.listeners.cmp(&y.listeners),
            Descending => y.listeners.cmp(&x.listeners),
        }
    });
}

fn show_feeds(feeds: &Vec<feed::Feed>, average_data: &AverageMap) -> Result<(), DetailedError> {
    #[cfg(unix)]
    let iter = feeds.iter().enumerate();
    #[cfg(windows)]
    let iter = feeds.iter().enumerate().rev();

    for (i, feed) in iter {
        let delta = average_data.get(&feed.id)
                        .map(|avg| avg.get_average_delta(feed.listeners as f32) as i32)
                        .unwrap_or(0);

        try_detailed!(notification::create_update,
            i as i32 + 1,
            feeds.len() as i32,
            &feed,
            delta);
    }

    Ok(())
}

fn perform_update(config: &Config, average_data: &mut AverageMap) -> Result<(), DetailedError> {
    let feeds = feed::get_latest(&config)?;
    let hour  = UTC::now().hour() as usize;

    let mut display_feeds = Vec::new();

    for feed in feeds {
        if feed.listeners < config.misc.minimum_listeners {
            continue
        }

        if cfg!(feature = "show-feed-info") {
            print!("{:?}\n^", feed);
        }

        let listeners = feed.listeners as f32;

        let listener_data =
            average_data
            .entry(feed.id)
            .or_insert(ListenerData::new(listeners, [0.; 24]));

        let has_spiked = listener_data.has_spiked(&config, &feed);
        listener_data.update(&config, hour, listeners, has_spiked);

        if has_spiked || feed.alert.is_some() {
            display_feeds.push(feed);
        }

        if cfg!(feature = "show-feed-info") {
            print!(" {:?} UNS: {:?}",
                listener_data.average,
                listener_data.unskewed_avg);

            if has_spiked {
                print!(" !!! SPIKED");
            }

            print!("\n\n");
        }
    }

    if display_feeds.len() > 0 {
        sort_feeds(&config, &mut display_feeds);
        show_feeds(&display_feeds, &average_data)?;
    }

    Ok(())
}

fn start() -> Result<(), DetailedError> {
    let config_path   = try_detailed!(util::verify_local_file, "config.yaml");
    let averages_path = try_detailed!(util::verify_local_file, "averages.csv");

    let mut listeners = listeners::load_averages(&averages_path)
        .unwrap_or(HashMap::new());

    let mut perform_cycle = || {
        let config = try_detailed!(config::load_from_file, &config_path);

        perform_update(&config, &mut listeners)?;
        try_detailed!(listeners::save_averages, &averages_path, &listeners);

        Ok(config)
    };

    loop {
        if cfg!(feature = "show-feed-info") {
            println!("updating");
        }

        let update_time_sec =
            match perform_cycle() {
                Ok(config) => config.misc.update_time as u64,
                Err(err) => {
                    error::report(&err);
                    config::Misc::default().update_time as u64
                },
            };

        thread::sleep(Duration::from_secs(update_time_sec * 60));
    }
}

fn main() {
    match start() {
        Ok(_) => (),
        Err(err) => {
            println!("FATAL ERROR:");
            error::report(&err);
        },
    }
}