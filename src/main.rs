#[macro_use]
extern crate diesel;

mod config;
mod database;
mod err;
mod feed;
mod path;

use crate::feed::stats::{ListenerAvgMap, ListenerStatMap, ListenerStats};
use crate::feed::{Feed, FeedNotif};
use chrono::{Timelike, Utc};
use clap::clap_app;
use config::Config;
use database::Database;
use diesel::prelude::*;
use err::Result;
use feed::stats::ListenerAvg;
use smallvec::SmallVec;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

fn main() {
    let args = clap_app!(bcnotif =>
        (version: env!("CARGO_PKG_VERSION"))
        (author: env!("CARGO_PKG_AUTHORS"))
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
    let db = Arc::new(Database::open()?);

    init_signal_handler(&db)?;

    let mut listener_avgs = ListenerAvgMap::with_capacity(200);
    let mut listener_stats = ListenerStatMap::with_capacity(200);

    let reload_config = args.is_present("RELOAD_CONFIG");

    loop {
        if reload_config {
            match Config::load() {
                Ok(new) => config = new,
                Err(err) => err::display_error(err),
            }
        }

        match run_update(&mut listener_stats, &mut listener_avgs, &db, &config) {
            Ok(mut notifs) => {
                FeedNotif::sort_all(&mut notifs, &config);

                if let Err(err) = FeedNotif::show_all(&notifs) {
                    err::display_error(err);
                }
            }
            Err(err) => err::display_error(err),
        };

        thread::sleep(Duration::from_secs((config.misc.update_time * 60.0) as u64));
    }
}

fn run_update<'a>(
    listener_stats: &mut ListenerStatMap,
    listener_avgs: &mut ListenerAvgMap,
    db: &Database,
    config: &Config,
) -> Result<SmallVec<[FeedNotif<'a>; 3]>> {
    use diesel::result::Error;

    let feeds = {
        let mut feeds = Feed::scrape_all(config)?;
        filter_feeds(config, &mut feeds);
        feeds
    };

    let hour = Utc::now().hour();
    let mut display = SmallVec::new();

    db.conn().transaction::<_, Error, _>(|| {
        for feed in feeds {
            let avg = listener_avgs
                .entry(feed.id)
                .or_insert_with(|| ListenerAvg::load_or_new(db, feed.id as i32));

            let stats = listener_stats.entry(feed.id).or_insert_with(|| {
                let listeners = avg.for_hour(hour).unwrap_or(feed.listeners as i32);
                ListenerStats::new(listeners as u32)
            });

            stats.update(&feed, config);

            avg.update_from_stats(hour, &stats);
            avg.save_to_db(db)?;

            if !stats.should_display_feed(&feed, config) {
                continue;
            }

            if display.len() > config.misc.max_feeds as usize {
                continue;
            }

            display.push(FeedNotif::new(feed, stats));
        }

        Ok(())
    })?;

    Ok(display)
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

fn init_signal_handler(db: &Arc<Database>) -> Result<()> {
    let db = db.clone();

    ctrlc::set_handler(move || {
        if let Err(err) = db.optimize() {
            err::display_error(err.into());
        }

        std::process::exit(0);
    })
    .map_err(Into::into)
}
