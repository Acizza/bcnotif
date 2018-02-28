extern crate reqwest;

pub mod statistics;
mod scrape;

use config::Config;
use failure::{Error, ResultExt};
use std::borrow::Cow;

#[derive(Debug)]
pub struct Feed<'a> {
    pub id: u32,
    pub name: String,
    pub listeners: u32,
    pub state: State<'a>,
    pub county: String,
    pub alert: Option<String>,
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

// TODO: move to scrape module (?)
#[derive(Fail, Debug)]
pub enum FeedSourceError {
    #[fail(display = "failed to parse top feeds")] FailedToParseTopFeeds,
    #[fail(display = "failed to parse state feeds")] FailedToParseStateFeeds,
}

enum FeedSource<'a> {
    Top,
    State(State<'a>),
}

impl<'a> FeedSource<'a> {
    fn get_url(&self) -> Cow<str> {
        match *self {
            FeedSource::Top => "http://www.broadcastify.com/listen/top".into(),
            FeedSource::State(ref state) => {
                format!("http://www.broadcastify.com/listen/stid/{}", state.id).into()
            }
        }
    }

    fn download_page(&self, client: &reqwest::Client) -> Result<String, Error> {
        let body = client.get(self.get_url().as_ref()).send()?.text()?;
        Ok(body)
    }

    fn scrape(self, client: &reqwest::Client) -> Result<Vec<Feed<'a>>, Error> {
        let body = self.download_page(client)?;

        match self {
            FeedSource::Top => {
                let scraped =
                    scrape::scrape_top(&body).context(FeedSourceError::FailedToParseTopFeeds)?;

                Ok(scraped)
            }
            FeedSource::State(state) => {
                let scraped = scrape::scrape_state(state, &body)
                    .context(FeedSourceError::FailedToParseStateFeeds)?;

                Ok(scraped)
            }
        }
    }
}

pub fn scrape_all(config: &Config) -> Result<Vec<Feed>, Error> {
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
