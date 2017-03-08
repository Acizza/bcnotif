pub mod listeners;
mod parse;

extern crate csv;
extern crate hyper;

use std::io::Read;
use config::Config;
use self::hyper::client::Client;

error_chain! {
    links {
        Parse(parse::Error, parse::ErrorKind);
    }
}

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

impl Feed {
    // TODO: Parse states from website (?)
    pub fn get_state_abbrev(&self) -> Option<&str> {
        macro_rules! create_matches {
            ($($id:expr => $abbrev:expr,)+) => {
                match self.state_id {
                    $($id => Some($abbrev),)+
                    _ => None,
                }
            };
        }

        create_matches!(
            1   => "AL", // Alabama
            2   => "AK", // Alaska
            4   => "AZ", // Arizona
            5   => "AR", // Arkansas
            6   => "CA", // California
            8   => "CO", // Colorado
            9   => "CT", // Connecticut
            10  => "DE", // Delaware
            11  => "DC", // District of Columbia
            12  => "FL", // Florida
            13  => "GA", // Georgia
            183 => "GU", // Guam
            15  => "HI", // Hawaii
            16  => "ID", // Idaho
            17  => "IL", // Illinois
            18  => "IN", // Indiana
            19  => "IA", // Iowa
            20  => "KS", // Kansas
            21  => "KY", // Kentucky
            22  => "LA", // Louisiana
            23  => "ME", // Maine
            24  => "MD", // Maryland
            25  => "MA", // Massachusetts
            26  => "MI", // Michigan
            27  => "MN", // Minnesota
            28  => "MS", // Mississippi
            29  => "MO", // Missouri
            30  => "MT", // Montana
            31  => "NE", // Nebraska
            32  => "NV", // Nevada
            33  => "NH", // New Hampshire
            34  => "NJ", // New Jersey
            35  => "NM", // New Mexico
            36  => "NY", // New York
            37  => "NC", // North Carolina
            38  => "ND", // North Dakota
            39  => "OH", // Ohio
            40  => "OK", // Oklahoma
            41  => "OR", // Oregon
            42  => "PA", // Pennsylvania
            57  => "PR", // Puerto Rico
            44  => "RI", // Rhode Island
            45  => "SC", // South Carolina
            46  => "SD", // South Dakota
            47  => "TN", // Tennessee
            48  => "TX", // Texas
            49  => "UT", // Utah
            50  => "VT", // Vermont
            181 => "VI", // Virgin Islands
            51  => "VA", // Virginia
            53  => "WA", // Washington
            54  => "WV", // West Virginia
            55  => "WI", // Wisconsin
            56  => "WY", // Wyoming
        )
    }
}

pub fn get_latest(config: &Config) -> Result<Vec<Feed>> {
    use self::FeedSource::*;
    
    let client = Client::new();
    let mut feeds = parse::top_feeds(&download_feed_data(&config, &client, Top)?)?;

    if let Some(id) = config.misc.state_feeds_id {
        feeds.extend(parse::state_feeds(
            &download_feed_data(&config, &client, State(id))?,
            id)?
        );
        
        // Remove any state feeds that show up in the top 50 list
        feeds.sort_by_key(|f| f.id);
        feeds.dedup();
    }

    filter(&config, &mut feeds);
    Ok(feeds)
}

fn download_feed_data(config: &Config, client: &Client, source: FeedSource) -> Result<String> {
    let url = match source {
        FeedSource::Top       => config.links.top_feeds.clone(),
        FeedSource::State(id) => format!("{}{}", &config.links.state_feeds, id),
    };

    let mut resp = client.get(&url).send()
        .chain_err(|| format!("failed to download feed data from {}", url))?;

    let mut body = String::new();

    resp.read_to_string(&mut body)
        .chain_err(|| format!("failed to read feed data into string from {}", url))?;

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