extern crate yaml_rust;

use std::error::Error;
use std::path::Path;
use chrono::{Local, Datelike};
use util;
use feed::Feed;
use self::yaml_rust::{YamlLoader, Yaml};

#[macro_use] mod macros;

create_config_enum!(FeedIdent,
    Name(String) => self,
    ID(u32)      => self,
    State(u32)   => "State ID",
);

impl FeedIdent {
    pub fn matches_feed(&self, feed: &Feed) -> bool {
        use self::FeedIdent::*;

        match *self {
            Name(ref name) => *name == feed.name,
            ID(id)         => id == feed.id,
            State(id)      => id == feed.state_id,
        }
    }
}

create_config_enum!(SortOrder,
    Ascending  => self,
    Descending => self,
);

create_config_enum!(WeekdaySpike,
    Sunday(Spike)    => self,
    Monday(Spike)    => self,
    Tuesday(Spike)   => self,
    Wednesday(Spike) => self,
    Thursday(Spike)  => self,
    Friday(Spike)    => self,
    Saturday(Spike)  => self,
);

impl WeekdaySpike {
    pub fn get_for_today(weekday_spikes: &[WeekdaySpike]) -> Option<&Spike> {
        use chrono::Weekday::*;
        use self::WeekdaySpike::*;

        let weekday = Local::today().weekday();

        for ws in weekday_spikes {
            match (weekday, ws) {
                (Mon, &Monday(ref s))    |
                (Tue, &Tuesday(ref s))   |
                (Wed, &Wednesday(ref s)) |
                (Thu, &Thursday(ref s))  |
                (Fri, &Friday(ref s))    |
                (Sat, &Saturday(ref s))  |
                (Sun, &Sunday(ref s)) => return Some(&s),
                _ => (),
            }
        }

        None
    }
}

create_config_struct!(Spike,
    jump:                    f32 => "Jump Required"                        => 0.25,
    low_listener_increase:   f32 => "Low Listener Increase"                => [0.0, 0.005],
    high_listener_dec:       f32 => "High Listener Decrease"               => [0.0, 0.02],
    high_listener_dec_every: f32 => "High Listener Decrease Per Listeners" => [1.0, 100.0],
);

create_config_struct!(FeedSetting,
    ident: FeedIdent => self                => fail,
    spike: Spike     => "Spike Percentages" => fail,
);

create_config_struct!(UnskewedAverage,
    reset_pcnt:      f32 => "Reset To Average Percentage"  => [0.0, 0.15],
    adjust_pcnt:     f32 => "Adjust to Average Percentage" => [0.0, 0.0075],
    spikes_required: u8  => "Listener Spikes Required"     => 1,
);

create_config_struct!(Misc,
	update_time:       f32         => "Update Time"       => [5.0, 6.0],
	minimum_listeners: u32         => "Minimum Listeners" => 15,
	state_feeds_id:    Option<u32> => "State Feeds ID"    => None,
    sort_order:        SortOrder   => "Feed Sort Order"   => (SortOrder::Descending),
);

create_config_struct!(Links,
    top_feeds:   String => "Top Feeds"   => ("http://broadcastify.com/listen/top".to_string()),
    state_feeds: String => "State Feeds" => ("http://www.broadcastify.com/listen/stid/".to_string()),
);

#[derive(Debug)]
pub struct Config {
    pub unskewed_avg: UnskewedAverage,
    pub misc:         Misc,
    pub links:        Links,
    pub blacklist:    Vec<FeedIdent>,
    pub whitelist:    Vec<FeedIdent>,
    global_spike:     Spike,
    weekday_spikes:   Vec<WeekdaySpike>,
    feed_settings:    Vec<FeedSetting>,
}

impl Config {
    pub fn get_current_spike(&self, feed: &Feed) -> &Spike {
        self.feed_settings
            .iter()
            .find(|setting| setting.ident.matches_feed(&feed))
            .map(|setting| &setting.spike)
            .unwrap_or({
                WeekdaySpike::get_for_today(&self.weekday_spikes)
                    .unwrap_or(&self.global_spike)
            })
    }
}

pub fn load_from_file(path: &Path) -> Result<Config, Box<Error>> {
    let doc = YamlLoader::load_from_str(&util::read_file(path)?)?;
    let doc = &doc[0]; // We don't care about multiple documents

    Ok(Config {
        unskewed_avg:   ParseYaml::from_or_default(&doc["Unskewed Average"]),
        misc:           ParseYaml::from_or_default(&doc["Misc"]),
        links:          ParseYaml::from_or_default(&doc["Source Links"]),
        blacklist:      ParseYaml::all(&doc["Blacklist"]),
        whitelist:      ParseYaml::all(&doc["Whitelist"]),
        global_spike:   ParseYaml::from_or_default(&doc["Spike Percentages"]),
        weekday_spikes: ParseYaml::all(&doc["Weekday Spike Percentages"]),
        feed_settings:  ParseYaml::all(&doc["Feed Settings"]),

    })
}

trait ParseYaml: Sized + Default {
    fn from(doc: &Yaml) -> Option<Self>;

    fn from_or_default(doc: &Yaml) -> Self {
        ParseYaml::from(&doc).unwrap_or(Self::default())
    }

    fn all(doc: &Yaml) -> Vec<Self> {
        doc.as_vec()
            .unwrap_or(&Vec::new())
            .iter()
            .filter_map(ParseYaml::from)
            .collect()
    }
}

macro_rules! impl_parseyaml_for_numeric {
    ($($t:ty )+) => {
        $(
        impl ParseYaml for $t {
            fn from(doc: &Yaml) -> Option<$t> {
                use self::yaml_rust::Yaml::*;
                match *doc {
                    Integer(num)     => Some(num as $t),
                    Real(ref string) => string.parse().ok(),
                    _                => None,
                }
            }
        }
        )+
    }
}

impl_parseyaml_for_numeric!(u8 u32 f32);

impl ParseYaml for String {
    fn from(doc: &Yaml) -> Option<String> {
        use self::yaml_rust::Yaml::*;
        match *doc {
            String(ref s) => Some(s.clone()),
            _             => None,
        }
    }
}