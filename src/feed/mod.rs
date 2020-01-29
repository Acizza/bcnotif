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

        if let Some(state_id) = config.misc.state_id {
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
pub struct FeedNotif<'a> {
    pub feed: Feed<'a>,
    pub jump: f32,
}

impl<'a> FeedNotif<'a> {
    pub fn new(feed: Feed<'a>, stats: &ListenerStats) -> Self {
        Self {
            feed,
            jump: stats.jump,
        }
    }

    pub fn show_notif(&self, index: u32, max_index: u32) -> Result<()> {
        let title = format!(
            concat!(env!("CARGO_PKG_NAME"), " update {} of {}"),
            index, max_index
        );

        let state = match &self.feed.location.state_name {
            Some(state) => state,
            None => "CS",
        };

        let alert = match &self.feed.alert {
            Some(alert) => Cow::Owned(format!("\nalert: {}", alert)),
            None => Cow::Borrowed(""),
        };

        let body = format!(
            "{state} | {name}\n{listeners} (^{jump}){alert}",
            state = state,
            name = self.feed.name,
            listeners = self.feed.listeners,
            jump = self.jump as i32,
            alert = alert,
        );

        Notification::new()
            .summary(&title)
            .body(&body)
            .show()
            .context(err::CreateNotif)
            .map(|_| ())
    }

    pub fn sort_all(notifs: &mut [Self], config: &Config) {
        use crate::config::{SortOrder, SortType};

        notifs.sort_unstable_by(|x, y| {
            let (x, y) = match config.sorting.order {
                SortOrder::Ascending => (x, y),
                SortOrder::Descending => (y, x),
            };

            match config.sorting.value {
                SortType::Listeners => x.feed.listeners.cmp(&y.feed.listeners),
                SortType::Jump => {
                    let x_jump = x.jump as i32;
                    let y_jump = y.jump as i32;

                    x_jump.cmp(&y_jump)
                }
            }
        });
    }

    pub fn show_all(notifs: &[Self]) -> Result<()> {
        let num_notifs = notifs.len() as u32;

        for (i, notif) in notifs.iter().enumerate() {
            notif.show_notif(1 + i as u32, num_notifs)?;
        }

        Ok(())
    }
}
