pub mod listeners;

extern crate csv;
extern crate hyper;
extern crate regex;

use std::error::Error;
use std::io::Read;
use config::{Config, FeedIdent};
use self::hyper::client::Client;
use self::regex::Regex;

enum FeedSource {
    Top,
    State(u8),
}

#[derive(Debug)]
pub struct Feed {
    pub id:        u32,
    pub name:      String,
    pub listeners: u32,
    pub alert:     Option<String>,
}

impl PartialEq for Feed {
    fn eq(&self, other: &Feed) -> bool {
        self.id == other.id
    }
}

fn parse(html: &str, source: FeedSource) -> Result<Vec<Feed>, Box<Error>> {
    lazy_static! {
        static ref TOP: Regex =
            Regex::new(
                r#"(?s)<td class="c m">(?P<listeners>\d+)</td>.+?/listen/feed/(?P<id>\d+)">(?P<name>.+?)</a>(?:<br /><br />.<div class="messageBox">(?P<alert>.+?)</div>)?"#)
                .unwrap();

        static ref STATE: Regex =
            Regex::new(
                r#"(?s)w1p">.+?<a href="/listen/feed/(?P<id>\d+)">(?P<name>.+?)</a>.+?(?:bold">(?P<alert>.+?)</font>.+?)?<td class="c m">(?P<listeners>\d+)</'td>"#)
                .unwrap();
    }

    let regex = match source {
        FeedSource::Top      => &*TOP,
        FeedSource::State(_) => &*STATE,
    };

    let mut feeds = Vec::new();

    for cap in regex.captures_iter(&html) {
        feeds.push(
            Feed {
                id:        cap["id"].parse()?,
                name:      cap["name"].to_string(),
                listeners: cap["listeners"].parse()?,
                alert:     cap.name("alert").map(|s| s.as_str().to_string()),
            }
        );
    }

    Ok(feeds)
}

fn download_feed_data(config: &Config, client: &Client, source: FeedSource) ->
    Result<String, Box<Error>> {

    let url = match source {
        FeedSource::Top       => config.links.top_feeds.clone(),
        FeedSource::State(id) => format!("{}{}", &config.links.state_feeds, id),
    };

    let mut resp = client.get(&url).send()?;

    let mut body = String::new();
    resp.read_to_string(&mut body)?;

    Ok(body)
}

fn filter(config: &Config, feeds: &mut Vec<Feed>) {
    use self::FeedIdent::*;

    if config.whitelist.len() > 0 {
        feeds.retain(|ref feed| {
            config.whitelist
                .iter()
                .any(|entry| {
                    match *entry {
                        Name(ref name) => &feed.name == name,
                        ID(id)         => feed.id == id,
                    }
                })
        });
    }

    for entry in &config.blacklist {
        let position = match *entry {
            Name(ref name) => feeds.iter().position(|ref feed| &feed.name == name),
            ID(id)         => feeds.iter().position(|ref feed| feed.id == id),
        };

        match position {
            Some(p) => {
                feeds.remove(p);
                ()
            },
            None => (),
        }
    }
}

pub fn get_latest(config: &Config) -> Result<Vec<Feed>, Box<Error>> {
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