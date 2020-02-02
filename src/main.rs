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
use config::Config;
use database::Database;
use diesel::prelude::*;
use err::Result;
use gumdrop::Options;
use smallvec::SmallVec;
use std::sync::{mpsc, Arc, Mutex};
use std::thread;

#[derive(Options)]
struct CmdOptions {
    #[options(help = "print help message")]
    help: bool,
    #[options(help = "reload the configuration file on each update")]
    reload_config: bool,
}

fn main() {
    let args = CmdOptions::parse_args_default_or_exit();

    match run(args) {
        Ok(_) => (),
        Err(err) => {
            err::display_error(err);
            std::process::exit(1);
        }
    }
}

enum Event {
    RunUpdate,
    Exit,
}

impl Event {
    fn init_threads(config: &Arc<Mutex<Config>>) -> Result<mpsc::Receiver<Self>> {
        let (tx, rx) = mpsc::channel();

        Self::spawn_update_thread(tx.clone(), config);
        Self::spawn_signal_handler(tx)?;

        Ok(rx)
    }

    fn spawn_update_thread(
        tx: mpsc::Sender<Self>,
        config: &Arc<Mutex<Config>>,
    ) -> thread::JoinHandle<()> {
        let config = config.clone();

        // This thread should die if something goes horribly wrong, so the uses of unwrap() are intended here
        thread::spawn(move || loop {
            let update_time = {
                let config = config.lock().unwrap();
                (config.misc.update_time_mins * 60.0) as u64
            };

            tx.send(Event::RunUpdate).unwrap();
            thread::sleep(std::time::Duration::from_secs(update_time));
        })
    }

    fn spawn_signal_handler(tx: mpsc::Sender<Self>) -> Result<()> {
        ctrlc::set_handler(move || {
            tx.send(Event::Exit).ok();
        })
        .map_err(Into::into)
    }
}

fn run(args: CmdOptions) -> Result<()> {
    let config = Arc::new(Mutex::new(Config::load_or_new()?));
    let db = Database::open()?;

    let mut listener_stats = ListenerStatMap::with_capacity(200);
    let mut remove_old_feeds_time = Utc::now();

    let event_rx = Event::init_threads(&config)?;

    loop {
        match event_rx.recv() {
            Ok(Event::RunUpdate) => {
                let cur_time = Utc::now();

                let mut config = match config.lock() {
                    Ok(config) => config,
                    Err(_) => {
                        err::display_error(err::Error::PoisonedMutex { name: "config" });
                        continue;
                    }
                };

                if args.reload_config {
                    match Config::load() {
                        Ok(new) => *config = new,
                        Err(err) => err::display_error(err),
                    }
                }

                let result = run_update(&db, &config, &cur_time, &mut listener_stats).and_then(
                    |mut notifs| {
                        FeedNotif::sort_all(&mut notifs, &config);
                        FeedNotif::show_all(&notifs)
                    },
                );

                if let Err(err) = result {
                    err::display_error(err);
                }

                if cur_time >= remove_old_feeds_time {
                    ListenerAvg::remove_old_from_db(&db)?;
                    remove_old_feeds_time = cur_time + Duration::hours(12);
                }
            }
            Ok(Event::Exit) => {
                if let Err(err) = db.optimize() {
                    err::display_error(err.into());
                }

                break Ok(());
            }
            Err(err) => break Err(err.into()),
        }
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
