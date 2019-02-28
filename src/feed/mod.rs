pub mod statistics;

mod scrape;

use crate::config::Config;
use crate::error::FeedError;
use crate::feed::statistics::ListenerStats;
use lazy_static::lazy_static;
use notify_rust::Notification;
use reqwest;
use std::borrow::Cow;

pub const BROADCASTIFY_URL: &str = "https://www.broadcastify.com";

#[derive(Debug)]
pub struct Feed<'a> {
    pub id: u32,
    pub name: String,
    pub listeners: u32,
    pub state: State<'a>,
    pub county: String,
    pub alert: Option<String>,
}

impl<'a> Feed<'a> {
    pub fn show_notification(
        &self,
        stats: &ListenerStats,
        index: u32,
        max_index: u32,
    ) -> Result<(), FeedError> {
        let title = format!(
            "{} - Broadcastify Update ({} of {})",
            self.state.abbrev, index, max_index
        );

        let alert = match self.alert {
            Some(ref alert) => Cow::Owned(format!("\nAlert: {}", alert)),
            None => Cow::Borrowed(""),
        };

        let body = format!(
            "Name: {name}\nListeners: {listeners} (^{jump}){alert}\nLink: {url}/listen/feed/{feed_id}",
            name = self.name,
            listeners = self.listeners,
            jump = stats.get_jump(self.listeners) as i32,
            alert = &alert,
            url = BROADCASTIFY_URL,
            feed_id = self.id
        );

        Notification::new()
            .summary(&title)
            .body(&body)
            .show()
            .map_err(|_| FeedError::FailedToCreateNotification)?;

        Ok(())
    }
}

impl<'a> PartialEq for Feed<'a> {
    fn eq(&self, other: &Feed) -> bool {
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

    fn scrape(self, client: &reqwest::Client) -> Result<Vec<Feed<'a>>, FeedError> {
        let body = self.download_page(client)?;

        match self {
            FeedSource::Top => scrape::scrape_top(&body).map_err(FeedError::ParseTopFeeds),
            FeedSource::State(ref state) => scrape::scrape_state(state, &body)
                .map_err(|e| FeedError::ParseStateFeeds(e, state.abbrev.to_string())),
        }
    }
}

pub fn scrape_all(config: &Config) -> Result<Vec<Feed>, FeedError> {
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

fn filter_whitelist_blacklist(config: &Config, feeds: &mut Vec<Feed>) {
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
