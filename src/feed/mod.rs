pub mod listeners;

extern crate csv;
extern crate hyper;
extern crate regex;

use std::io::Read;
use config::Config;
use util::error::DetailedError;
use self::hyper::client::Client;
use self::regex::Regex;

enum FeedSource {
    Top,
    State(u32),
}

#[derive(Debug)]
pub struct Feed {
    pub id:        u32,
    pub state_id:  u32,
    pub county:    String,
    pub name:      String,
    pub listeners: u32,
    pub alert:     Option<String>,
}

impl PartialEq for Feed {
    fn eq(&self, other: &Feed) -> bool {
        self.id == other.id
    }
}

fn parse(html: &str, source: FeedSource) -> Result<Vec<Feed>, DetailedError> {
    lazy_static! {
        static ref TOP: Regex =
            Regex::new(
                r#"(?s)<td class="c m">(?P<listeners>\d+)</td>.+?/listen/stid/(?P<state_id>\d+)">(?:.+?/listen/ctid/\d+">(?P<county>.+?)</a>)*?.+?/listen/feed/(?P<id>\d+)">(?P<name>.+?)</a>(?:<br /><br />.<div class="messageBox">(?P<alert>.+?)</div>)?"#)
                .unwrap();

        static ref STATE: Regex =
            Regex::new(
                r#"(?s)listen/ctid/\d+">(?P<county>.+?)</a>.+?w1p">.+?<a href="/listen/feed/(?P<id>\d+)">(?P<name>.+?)</a>.+?(?:bold">(?P<alert>.+?)</font>.+?)?<td class="c m">(?P<listeners>\d+)</'td>"#)
                .unwrap();
    }

    let regex = match source {
        FeedSource::Top      => &*TOP,
        FeedSource::State(_) => &*STATE,
    };

    let mut feeds = Vec::new();

    for cap in regex.captures_iter(&html) {
        let state_id = match source {
            FeedSource::Top       => try_detailed!(cap["state_id"].parse::<u32>()),
            FeedSource::State(id) => id,
        };

        feeds.push(
            Feed {
                id:        try_detailed!(cap["id"].parse()),
                state_id:  state_id,
                county:    cap.name("county")
                              .map(|s| s.as_str())
                              .unwrap_or("Numerous")
                              .to_string(),
                name:      cap["name"].to_string(),
                listeners: try_detailed!(cap["listeners"].parse()),
                alert:     cap.name("alert").map(|s| s.as_str().to_string()),
            }
        );
    }

    Ok(feeds)
}

fn download_feed_data(config: &Config, client: &Client, source: FeedSource) ->
    Result<String, DetailedError> {

    let url = match source {
        FeedSource::Top       => config.links.top_feeds.clone(),
        FeedSource::State(id) => format!("{}{}", &config.links.state_feeds, id),
    };

    let mut resp = try_detailed!(client.get(&url).send());

    let mut body = String::new();
    try_detailed!(resp.read_to_string(&mut body));

    Ok(body)
}

fn filter(config: &Config, feeds: &mut Vec<Feed>) {
    if config.whitelist.len() > 0 {
        feeds.retain(|ref feed| {
            config.whitelist
                .iter()
                .any(|entry| entry.matches_feed(&feed))
        });
    }

    if config.blacklist.len() > 0 {
        feeds.retain(|ref feed| {
            config.blacklist
                .iter()
                .any(|entry| !entry.matches_feed(&feed))
        });
    }
}

pub fn get_latest(config: &Config) -> Result<Vec<Feed>, DetailedError> {
    use self::FeedSource::*;
    
    let client = Client::new();
    let mut feeds = parse(&download_feed_data(&config, &client, Top)?, Top)?;

    if let Some(id) = config.misc.state_feeds_id {
        feeds.extend(parse(&download_feed_data(&config, &client, State(id))?, State(id))?);
        
        // Remove any state feeds that show up in the top 50 list
        feeds.sort_by_key(|f| f.id);
        feeds.dedup();
    }

    filter(&config, &mut feeds);
    Ok(feeds)
}