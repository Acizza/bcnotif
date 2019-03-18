pub mod stats;

mod scrape;

use crate::config::Config;
use crate::error::FeedError;
use crate::path;
use hashbrown::HashMap;
use lazy_static::lazy_static;
use notify_rust::Notification;
use reqwest;
use stats::ListenerStats;
use std::borrow::Cow;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::mem;
use std::path::PathBuf;

pub const BROADCASTIFY_URL: &str = "https://www.broadcastify.com";

type FeedID = u32;

#[derive(Debug)]
pub struct FeedInfo<'a> {
    pub id: FeedID,
    pub name: String,
    pub listeners: u32,
    pub state: State<'a>,
    pub county: String,
    pub alert: Option<String>,
}

impl<'a> PartialEq for FeedInfo<'a> {
    fn eq(&self, other: &FeedInfo) -> bool {
        self.id == other.id
    }
}

#[derive(Debug, Clone)]
pub struct State<'a> {
    pub id: u32,
    pub abbrev: Cow<'a, str>,
}

impl<'a> State<'a> {
    pub fn new<S>(id: u32, abbrev: S) -> State<'a>
    where
        S: Into<Cow<'a, str>>,
    {
        State {
            id,
            abbrev: abbrev.into(),
        }
    }
}

pub enum FeedSource<'a> {
    Top,
    State(State<'a>),
}

impl<'a> FeedSource<'a> {
    fn get_url(&self) -> String {
        match *self {
            FeedSource::Top => {
                // This should be be converted to use the concat! macro if it
                // ever gains support for constants
                format!("{}/listen/top", BROADCASTIFY_URL)
            }
            FeedSource::State(ref state) => {
                format!("{}/listen/stid/{}", BROADCASTIFY_URL, state.id)
            }
        }
    }

    fn download_page(&self, client: &reqwest::Client) -> reqwest::Result<String> {
        let body = client.get(&self.get_url()).send()?.text()?;
        Ok(body)
    }

    fn scrape(self, client: &reqwest::Client) -> Result<Vec<FeedInfo<'a>>, FeedError> {
        let body = self.download_page(client)?;

        match self {
            FeedSource::Top => scrape::scrape_top(&body).map_err(FeedError::ParseTopFeeds),
            FeedSource::State(ref state) => scrape::scrape_state(state, &body)
                .map_err(|e| FeedError::ParseStateFeeds(e, state.abbrev.to_string())),
        }
    }
}

pub fn scrape_all(config: &Config) -> Result<Vec<FeedInfo>, FeedError> {
    lazy_static! {
        static ref CLIENT: reqwest::Client = reqwest::Client::new();
    }

    let mut feeds = FeedSource::Top.scrape(&CLIENT)?;

    if let Some(state_id) = config.misc.state_feeds_id {
        let state = State::new(state_id, "CS"); // CS = Config Specified
        let state_feeds = FeedSource::State(state).scrape(&CLIENT)?;

        feeds.extend(state_feeds);
    }

    filter_whitelist_blacklist(config, &mut feeds);

    feeds.sort_by_key(|feed| feed.id);
    feeds.dedup();

    Ok(feeds)
}

fn filter_whitelist_blacklist(config: &Config, feeds: &mut Vec<FeedInfo>) {
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

#[derive(Debug)]
pub struct FeedData {
    pub path: PathBuf,
    pub stats: HashMap<FeedID, ListenerStats>,
}

impl FeedData {
    pub const DEFAULT_FNAME: &'static str = "averages.csv";

    pub fn new<P>(path: P) -> FeedData
    where
        P: Into<PathBuf>,
    {
        FeedData {
            path: path.into(),
            stats: HashMap::new(),
        }
    }

    pub fn default_path() -> Result<PathBuf, FeedError> {
        let path = path::get_data_file(FeedData::DEFAULT_FNAME)?;
        Ok(path)
    }

    pub fn load<P>(path: P) -> Result<FeedData, FeedError>
    where
        P: Into<PathBuf>,
    {
        let path = path.into();
        let reader = BufReader::new(File::open(&path)?);
        let mut stats = HashMap::with_capacity(1000);

        for line in reader.lines() {
            let line = line?;
            let split = line.split(',').collect::<Vec<&str>>();

            if split.len() != 1 + stats::NUM_HOURLY_STATS {
                return Err(FeedError::MalformedCSV);
            }

            let id = split[0].parse().map_err(|_| FeedError::MalformedCSV)?;

            let average_hourly = unsafe {
                let mut arr: [f32; stats::NUM_HOURLY_STATS] = mem::uninitialized();

                for (i, val) in split[1..=stats::NUM_HOURLY_STATS].iter().enumerate() {
                    arr[i] = val.parse().map_err(|_| FeedError::MalformedCSV)?;
                }

                arr
            };

            let stat = ListenerStats::with_hourly(average_hourly);
            stats.insert(id, stat);
        }

        let feed_data = FeedData { path, stats };
        Ok(feed_data)
    }

    pub fn save(&self) -> Result<(), FeedError> {
        let mut buffer = String::new();

        for (id, stats) in &self.stats {
            {
                let id = id.to_string();
                buffer.reserve(id.len() + 1);
                buffer.push_str(&id);
            }

            let hourly = stats.average_hourly;

            for stat in &hourly {
                let stat = (*stat as i32).to_string();
                buffer.reserve(stat.len() + 1);
                buffer.push(',');
                buffer.push_str(&stat);
            }

            buffer.push('\n');
        }

        std::fs::write(&self.path, buffer)?;

        Ok(())
    }
}

#[derive(Debug)]
pub struct FeedDisplay<'a> {
    pub info: FeedInfo<'a>,
    pub jump: f32,
    pub spike_count: u32,
    pub has_spiked: bool,
}

impl<'a> FeedDisplay<'a> {
    pub fn from(info: FeedInfo<'a>, stats: &ListenerStats) -> FeedDisplay<'a> {
        let jump = stats.get_jump(info.listeners);

        FeedDisplay {
            info,
            jump,
            spike_count: stats.spike_count,
            has_spiked: stats.has_spiked,
        }
    }

    pub fn show_notif(&self, index: u32, max_index: u32) -> Result<(), FeedError> {
        let title = format!(
            "{} update {} of {}",
            env!("CARGO_PKG_NAME"),
            index,
            max_index
        );

        let alert = match &self.info.alert {
            Some(alert) => Cow::Owned(format!("\nalert: {}", alert)),
            None => Cow::Borrowed(""),
        };

        let body = format!(
            "{state} | {name}\n{listeners} (^{jump}){alert}",
            state = self.info.state.abbrev,
            name = self.info.name,
            listeners = self.info.listeners,
            jump = self.jump as i32,
            alert = &alert,
        );

        Notification::new()
            .summary(&title)
            .body(&body)
            .show()
            .map_err(|_| FeedError::FailedToCreateNotification)?;

        Ok(())
    }
}

pub fn should_be_displayed(feed: &FeedInfo, stats: &ListenerStats, config: &Config) -> bool {
    if let Some(max_times) = config.misc.max_times_to_show_feed {
        if stats.spike_count > max_times {
            return false;
        }
    }

    let has_alert = feed.alert.is_some() && config.misc.show_alert_feeds;
    stats.has_spiked || has_alert
}
