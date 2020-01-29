#[macro_use]
extern crate diesel;

mod config;
mod database;
mod err;
mod feed;
mod path;

use crate::feed::stats::{ListenerAvg, ListenerStatMap, ListenerStats};
use crate::feed::{Feed, FeedNotif};
use chrono::{DateTime, Datelike, Duration, Local, Timelike, Utc};
use clap::clap_app;
use config::Config;
use database::Database;
use diesel::prelude::*;
use err::Result;
use smallvec::SmallVec;
use std::sync::Arc;
use std::thread;

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
    let mut config = Config::load_or_new()?;
    let db = Arc::new(Database::open()?);

    init_signal_handler(&db)?;

    let mut listener_stats = ListenerStatMap::with_capacity(200);
    let mut remove_old_feeds_time = Utc::now();
    let reload_config = args.is_present("RELOAD_CONFIG");

    loop {
        let cur_time = Utc::now();

        if reload_config {
            match Config::load() {
                Ok(new) => config = new,
                Err(err) => err::display_error(err),
            }
        }

        match run_update(&db, &config, &cur_time, &mut listener_stats) {
            Ok(mut notifs) => {
                FeedNotif::sort_all(&mut notifs, &config);

                if let Err(err) = FeedNotif::show_all(&notifs) {
                    err::display_error(err);
                }
            }
            Err(err) => err::display_error(err),
        };

        if cur_time >= remove_old_feeds_time {
            ListenerAvg::remove_old_from_db(&db)?;
            remove_old_feeds_time = cur_time + Duration::hours(12);
        }

        // Account for time drift so we always get updates at predictable times
        let update_time =
            cur_time + Duration::seconds((config.misc.update_time_mins * 60.0) as i64);

        let sleep_time = update_time
            .signed_duration_since(Utc::now())
            .to_std()
            .unwrap_or_else(|_| std::time::Duration::from_secs(5 * 60));

        thread::sleep(sleep_time);
    }
}

fn run_update<'a>(
    db: &Database,
    config: &Config,
    cur_time: &DateTime<Utc>,
    listener_stats: &mut ListenerStatMap,
) -> Result<SmallVec<[FeedNotif<'a>; 3]>> {
    use diesel::result::Error;

    let feeds = {
        let mut feeds = Feed::scrape_all(config)?;
        filter_feeds(config, &mut feeds);
        feeds
    };

    let cur_hour = cur_time.hour() as u8;
    let cur_weekday = Local::today().weekday();

    let mut display = SmallVec::new();

    db.conn().transaction::<_, Error, _>(|| {
        for feed in feeds {
            let stats = listener_stats.entry(feed.id).or_insert_with(|| {
                ListenerStats::init_from_db(db, cur_hour, feed.id as i32, feed.listeners as f32)
            });

            stats.update(cur_hour, &feed, config, cur_weekday);
            stats.save_to_db(db)?;

            if !stats.should_display_feed(&feed, config) {
                continue;
            }

            if display.len() > config.misc.show_max as usize {
                continue;
            }

            display.push(FeedNotif::new(feed, stats));
        }

        Ok(())
    })?;

    Ok(display)
}

fn filter_feeds(config: &Config, feeds: &mut Vec<Feed>) {
    if !config.filters.whitelist.is_empty() {
        feeds.retain(|feed| {
            config
                .filters
                .whitelist
                .iter()
                .any(|entry| entry.matches_feed(feed))
        });
    }

    if !config.filters.blacklist.is_empty() {
        feeds.retain(|feed| {
            config
                .filters
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
