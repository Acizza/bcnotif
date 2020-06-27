#[macro_use]
extern crate diesel;
#[macro_use]
extern crate num_derive;

mod config;
mod database;
mod err;
mod feed;
mod path;

use crate::feed::stats::{ListenerAvg, ListenerStatMap, ListenerStats};
use crate::feed::{Feed, FeedNotif};
use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Datelike, Duration, Local, Timelike, Utc};
use config::Config;
use database::Database;
use diesel::prelude::*;
use once_cell::sync::Lazy;
use parking_lot::{Condvar, Mutex};
use smallvec::SmallVec;
use std::sync::{mpsc, Arc};
use std::thread;

struct CmdOptions {
    reload_config: bool,
}

impl CmdOptions {
    fn from_env() -> Self {
        let mut args = pico_args::Arguments::from_env();

        if args.contains(["-h", "--help"]) {
            Self::print_help();
        }

        Self {
            reload_config: args.contains(["-r", "--reload"]),
        }
    }

    fn print_help() {
        println!(concat!("Usage: ", env!("CARGO_PKG_NAME"), " [OPTIONS]\n"));

        println!("Optional arguments:");
        println!("  -h, --help    show this message");
        println!("  -r, --reload  reload the configuration file on each update");

        std::process::exit(0);
    }
}

fn main() -> Result<()> {
    let args = CmdOptions::from_env();
    let result = run(args);

    if let Err(err) = &result {
        err::error_notif(err);
    }

    result
}

fn run(args: CmdOptions) -> Result<()> {
    let config = {
        let cfg = Config::load_or_new().context("failed to load / create config")?;
        Arc::new(Mutex::new(cfg))
    };

    let db = Database::open().context("failed to open feed database")?;

    let mut listener_stats = ListenerStatMap::with_capacity(200);
    let mut remove_old_feeds_time = Utc::now();

    let event_rx = Event::init_threads(&config).context("failed to init event threads")?;

    loop {
        match event_rx.recv() {
            Ok(Event::RunUpdate) => {
                let cur_time = Utc::now();
                let mut config = config.lock();

                if args.reload_config {
                    match Config::load() {
                        Ok(new) => *config = new,
                        Err(err) => err::error_notif(&err),
                    }
                }

                let result = run_update(&db, &config, &cur_time, &mut listener_stats).and_then(
                    |mut notifs| {
                        FeedNotif::sort_all(&mut notifs, &config);
                        FeedNotif::show_all(&notifs)
                    },
                );

                if let Err(err) = result {
                    err::error_notif(&err);
                }

                if cur_time >= remove_old_feeds_time {
                    ListenerAvg::remove_old_from_db(&db)?;
                    remove_old_feeds_time = cur_time + Duration::hours(12);
                }
            }
            Ok(Event::Exit) => break Ok(()),
            Err(err) => break Err(err.into()),
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
        Self::spawn_signal_handler(tx).context("signal handler spawn failed")?;

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
                let config = config.lock();
                (config.misc.update_time_mins * 60.0) as u64
            };

            if tx.send(Event::RunUpdate).is_err() {
                break;
            }

            thread::sleep(std::time::Duration::from_secs(update_time));
        })
    }

    fn spawn_signal_handler(tx: mpsc::Sender<Self>) -> Result<()> {
        use nix::sys::signal::{signal, SigHandler, Signal};

        static SIG_TRIGGER: Lazy<(Mutex<()>, Condvar)> =
            Lazy::new(|| (Mutex::new(()), Condvar::new()));

        extern "C" fn handle_sig(_: libc::c_int) {
            let (_, cvar) = &*SIG_TRIGGER;
            cvar.notify_one();
        }

        let handler = SigHandler::Handler(handle_sig);
        let sigs = [Signal::SIGHUP, Signal::SIGTERM, Signal::SIGINT];

        unsafe {
            for &sig in &sigs {
                signal(sig, handler)
                    .map_err(|err| anyhow!("failed to register signal handler: {}", err))?;
            }
        }

        thread::spawn(move || {
            let (lock, cvar) = &*SIG_TRIGGER;
            cvar.wait(&mut lock.lock());
            tx.send(Event::Exit).ok();
        });

        Ok(())
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
        let mut feeds = Feed::scrape_all(config).context("feed scraping failed")?;
        filter_feeds(config, &mut feeds);
        feeds
    };

    let cur_hour = cur_time.hour() as u8;
    let cur_weekday = Local::today().weekday();

    let mut display = SmallVec::new();

    db.conn()
        .transaction::<_, Error, _>(|| {
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
        })
        .context("database transaction failed")?;

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
