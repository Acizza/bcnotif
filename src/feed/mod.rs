pub mod stats;

mod scrape;

use crate::config::Config;
use crate::err::{self, Result};
use notify_rust::Notification;
use once_cell::sync::Lazy;
use reqwest::blocking::Client;
use snafu::ResultExt;
use stats::ListenerStats;
use std::borrow::Cow;
use std::cmp::{self, Eq, Ord};

#[derive(Debug)]
pub struct Feed<'a> {
    pub id: u32,
    pub name: String,
    pub listeners: u32,
    pub location: Location,
    pub county: Cow<'a, str>,
    pub alert: Option<String>,
}

impl<'a> Feed<'a> {
    pub fn scrape_all(config: &Config) -> Result<Vec<Self>> {
        let mut feeds = Self::scrape_source(Source::Top50, config.misc.minimum_listeners)?;

        if let Some(state_id) = config.misc.state_feeds_id {
            let state_feeds =
                Self::scrape_source(Source::State(state_id), config.misc.minimum_listeners)?;

            feeds.extend(state_feeds);
        }

        feeds.sort_unstable();
        feeds.dedup();

        Ok(feeds)
    }

    fn scrape_source(source: Source, min_listeners: u32) -> Result<Vec<Self>> {
        static CLIENT: Lazy<Client> = Lazy::new(Client::new);

        let body = CLIENT.get(source.url().as_ref()).send()?.text()?;

        match source {
            Source::Top50 => scrape::scrape_top(&body, min_listeners).context(err::ParseTopFeeds),
            Source::State(id) => {
                scrape::scrape_state(&body, min_listeners, id).context(err::ParseStateFeeds)
            }
        }
    }
}

impl<'a> PartialEq for Feed<'a> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl<'a> Eq for Feed<'a> {}

impl<'a> PartialOrd for Feed<'a> {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<'a> Ord for Feed<'a> {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        self.id.cmp(&other.id)
    }
}

#[derive(Debug)]
pub struct Location {
    pub state_id: u32,
    pub state_name: Option<String>,
}

impl Location {
    pub fn with_state<S>(state_id: u32, state_name: S) -> Self
    where
        S: Into<String>,
    {
        Self {
            state_id,
            state_name: Some(state_name.into()),
        }
    }

    pub fn new(state_id: u32) -> Self {
        Self {
            state_id,
            state_name: None,
        }
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
pub struct FeedDisplay<'a> {
    pub feed: Feed<'a>,
    pub jump: f32,
    pub spike_count: u32,
    pub has_spiked: bool,
}

impl<'a> FeedDisplay<'a> {
    pub fn from(feed: Feed<'a>, stats: &ListenerStats) -> Self {
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

        let state = match &self.feed.location.state_name {
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
