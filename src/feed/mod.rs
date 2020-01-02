pub mod stats;

mod scrape;

use crate::config::Config;
use crate::err::{self, Result};
use crate::path;
use notify_rust::Notification;
use once_cell::sync::Lazy;
use reqwest::blocking::Client;
use snafu::{OptionExt, ResultExt};
use stats::ListenerStats;
use std::borrow::Cow;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::mem;
use std::path::PathBuf;

type FeedID = u32;

#[derive(Debug)]
pub struct FeedInfo {
    pub id: FeedID,
    pub name: String,
    pub listeners: u32,
    pub location: Location,
    pub county: String,
    pub alert: Option<String>,
}

impl FeedInfo {
    pub fn scrape_from_source(source: FeedSource) -> Result<Vec<FeedInfo>> {
        static CLIENT: Lazy<Client> = Lazy::new(Client::new);

        let body = CLIENT.get(source.as_url_str().as_ref()).send()?.text()?;

        match source {
            FeedSource::Top50 => scrape::scrape_top(&body).context(err::ParseTopFeeds),
            FeedSource::State(id) => scrape::scrape_state(id, &body).context(err::ParseStateFeeds),
        }
    }

    pub fn scrape_from_config(config: &Config) -> Result<Vec<FeedInfo>> {
        let mut feeds = FeedInfo::scrape_from_source(FeedSource::Top50)?;

        if let Some(state_id) = config.misc.state_feeds_id {
            let state_feeds = FeedInfo::scrape_from_source(FeedSource::State(state_id))?;
            feeds.reserve(state_feeds.len());
            feeds.extend(state_feeds);
        }

        feeds.sort_unstable_by_key(|feed| feed.id);
        feeds.dedup();

        Ok(feeds)
    }
}

impl PartialEq for FeedInfo {
    fn eq(&self, other: &FeedInfo) -> bool {
        self.id == other.id
    }
}

#[derive(Debug)]
pub enum Location {
    FromTop50(u32, String),
    FromState(u32),
}

pub enum FeedSource {
    Top50,
    State(u32),
}

impl FeedSource {
    pub fn as_url_str(&self) -> Cow<str> {
        match self {
            FeedSource::Top50 => Cow::Borrowed("https://www.broadcastify.com/listen/top"),
            FeedSource::State(id) => {
                format!("https://www.broadcastify.com/listen/stid/{}", id).into()
            }
        }
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

    pub fn default_path() -> Result<PathBuf> {
        let path = path::get_data_file(FeedData::DEFAULT_FNAME)?;
        Ok(path)
    }

    pub fn load<P>(path: P) -> Result<FeedData>
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
                return Err(err::Error::MalformedCSV);
            }

            let id = split[0].parse().ok().context(err::MalformedCSV)?;

            let average_hourly = unsafe {
                let mut arr: [f32; stats::NUM_HOURLY_STATS] = mem::uninitialized();

                for (i, val) in split[1..=stats::NUM_HOURLY_STATS].iter().enumerate() {
                    arr[i] = val.parse().ok().context(err::MalformedCSV)?;
                }

                arr
            };

            let stat = ListenerStats::with_hourly(average_hourly);
            stats.insert(id, stat);
        }

        let feed_data = FeedData { path, stats };
        Ok(feed_data)
    }

    pub fn save(&self) -> Result<()> {
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
pub struct FeedDisplay {
    pub info: FeedInfo,
    pub jump: f32,
    pub spike_count: u32,
    pub has_spiked: bool,
}

impl FeedDisplay {
    pub fn from(info: FeedInfo, stats: &ListenerStats) -> FeedDisplay {
        let jump = stats.get_jump(info.listeners);

        FeedDisplay {
            info,
            jump,
            spike_count: stats.spike_count,
            has_spiked: stats.has_spiked,
        }
    }

    pub fn show_notif(&self, index: u32, max_index: u32) -> Result<()> {
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

        let state = match &self.info.location {
            Location::FromTop50(_, name) => name.as_ref(),
            Location::FromState(_) => "CS",
        };

        let body = format!(
            "{state} | {name}\n{listeners} (^{jump}){alert}",
            state = state,
            name = self.info.name,
            listeners = self.info.listeners,
            jump = self.jump as i32,
            alert = &alert,
        );

        Notification::new()
            .summary(&title)
            .body(&body)
            .show()
            .context(err::CreateNotif)?;

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
