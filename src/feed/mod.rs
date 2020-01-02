pub mod stats;

mod scrape;

use crate::config::Config;
use crate::err::{self, Result};
use crate::path::FilePath;
use notify_rust::Notification;
use once_cell::sync::Lazy;
use reqwest::blocking::Client;
use snafu::{OptionExt, ResultExt};
use stats::ListenerStats;
use std::borrow::Cow;
use std::cmp::{self, Eq, Ord};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::mem;
use std::path::PathBuf;

#[derive(Debug)]
pub struct Feed {
    pub id: u32,
    pub name: String,
    pub listeners: u32,
    pub location: Location,
    pub county: String,
    pub alert: Option<String>,
}

impl Feed {
    fn scrape_all_source(source: Source) -> Result<Vec<Self>> {
        static CLIENT: Lazy<Client> = Lazy::new(Client::new);

        let body = CLIENT.get(source.url().as_ref()).send()?.text()?;

        match source {
            Source::Top50 => scrape::scrape_top(&body).context(err::ParseTopFeeds),
            Source::State(id) => scrape::scrape_state(id, &body).context(err::ParseStateFeeds),
        }
    }

    pub fn scrape_all(config: &Config) -> Result<Vec<Self>> {
        let mut feeds = Self::scrape_all_source(Source::Top50)?;

        if let Some(state_id) = config.misc.state_feeds_id {
            let state_feeds = Self::scrape_all_source(Source::State(state_id))?;
            feeds.extend(state_feeds);
        }

        feeds.sort_unstable();
        feeds.dedup();

        Ok(feeds)
    }
}

impl PartialEq for Feed {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for Feed {}

impl PartialOrd for Feed {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Feed {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        self.id.cmp(&other.id)
    }
}

#[derive(Debug)]
pub struct Location {
    pub id: u32,
    pub state: Option<String>,
}

impl Location {
    pub fn with_state<S>(id: u32, state: S) -> Self
    where
        S: Into<String>,
    {
        Self {
            id,
            state: Some(state.into()),
        }
    }

    pub fn new(id: u32) -> Self {
        Self { id, state: None }
    }
}

pub type StateID = u32;

pub enum Source {
    Top50,
    State(StateID),
}

impl Source {
    pub fn url(&self) -> Cow<str> {
        match self {
            Self::Top50 => "https://www.broadcastify.com/listen/top".into(),
            Self::State(id) => format!("https://www.broadcastify.com/listen/stid/{}", id).into(),
        }
    }
}

#[derive(Debug)]
pub struct FeedData {
    pub path: PathBuf,
    pub stats: HashMap<u32, ListenerStats>,
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
        let mut path = FilePath::LocalData.validated_dir_path()?;
        path.push(Self::DEFAULT_FNAME);
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
    pub feed: Feed,
    pub jump: f32,
    pub spike_count: u32,
    pub has_spiked: bool,
}

impl FeedDisplay {
    pub fn from(feed: Feed, stats: &ListenerStats) -> FeedDisplay {
        let jump = stats.get_jump(feed.listeners);

        FeedDisplay {
            feed,
            jump,
            spike_count: stats.spike_count,
            has_spiked: stats.has_spiked,
        }
    }

    pub fn show_notif(&self, index: u32, max_index: u32) -> Result<()> {
        let title = format!(
            concat!(env!("CARGO_PKG_NAME"), " update {} of {}"),
            index, max_index
        );

        let alert = match &self.feed.alert {
            Some(alert) => Cow::Owned(format!("\nalert: {}", alert)),
            None => Cow::Borrowed(""),
        };

        let state = match &self.feed.location.state {
            Some(state) => state,
            None => "CS",
        };

        let body = format!(
            "{state} | {name}\n{listeners} (^{jump}){alert}",
            state = state,
            name = self.feed.name,
            listeners = self.feed.listeners,
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

pub fn should_be_displayed(feed: &Feed, stats: &ListenerStats, config: &Config) -> bool {
    if let Some(max_times) = config.misc.max_times_to_show_feed {
        if stats.spike_count > max_times {
            return false;
        }
    }

    let has_alert = feed.alert.is_some() && config.misc.show_alert_feeds;
    stats.has_spiked || has_alert
}
