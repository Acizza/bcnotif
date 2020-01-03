#[macro_use]
mod generation;

use crate::err::Result;
use crate::feed::Feed;
use crate::path::FilePath;
use chrono::{Datelike, Local};
use std::path::Path;
use yaml_rust::{Yaml, YamlLoader};

pub const DEFAULT_CONFIG_NAME: &str = "config.yaml";

macro_rules! gen_base_parse_stmt {
    (optional, $category:expr, $doc:ident) => {
        ParseYaml::from(&$doc[$category])
    };
    (default, $category:expr, $doc:ident) => {
        ParseYaml::from_or_default(&$doc[$category])
    };
    (all, $category:expr, $doc:ident) => {
        ParseYaml::all(&$doc[$category])
    };
}

macro_rules! gen_base_config {
    ($name:ident, $($field:ident: $type:ty => $parse_type:ident => $category:expr,)+) => {
        #[derive(Debug, Default)]
        pub struct $name {
            $(pub $field: $type,)+
        }

        impl $name {
            pub fn from_file(path: &Path) -> Result<$name> {
                let file = ::std::fs::read_to_string(path)?;

                if file.len() == 0 {
                    return Ok(Config::default())
                }

                let doc = YamlLoader::load_from_str(&file)?;
                let doc = &doc[0]; // We only care about the first document

                Ok($name {
                    $($field: gen_base_parse_stmt!($parse_type, $category, doc),)+
                })
            }
        }
    };
}

gen_base_config!(Config,
    global_spike:   Spike             => default => "Spike Percentage",
    unskewed_avg:   UnskewedAverage   => default => "Unskewed Average",
    weekday_spikes: Vec<WeekdaySpike> => all     => "Weekday Spike Percentages",
    feed_settings:  Vec<FeedSetting>  => all     => "Feed Settings",
    misc:           Misc              => default => "Misc",
    sorting:        Sorting           => default => "Feed Sorting",
    blacklist:      Vec<FeedIdent>    => all     => "Blacklist",
    whitelist:      Vec<FeedIdent>    => all     => "Whitelist",
);

impl Config {
    pub fn load() -> Result<Config> {
        let mut path = FilePath::Config.validated_dir_path()?;
        path.push(DEFAULT_CONFIG_NAME);

        if path.exists() {
            Config::from_file(&path)
        } else {
            Ok(Config::default())
        }
    }

    /// Gets the spike values for the specified feed based off of
    /// other configuration values that may be set.
    pub fn get_feed_spike(&self, feed: &Feed) -> &Spike {
        // Find any settings for the specified feed
        let feed_setting = self
            .feed_settings
            .iter()
            .find(|s| s.ident.matches_feed(feed));

        if let Some(setting) = feed_setting {
            WeekdaySpike::get_for_today(&setting.weekday_spikes).unwrap_or(&setting.spike)
        } else {
            WeekdaySpike::get_for_today(&self.weekday_spikes).unwrap_or(&self.global_spike)
        }
    }
}

create_config_struct!(Spike,
    jump:                    f32 => "Jump Required"                        => 0.4,
    low_listener_increase:   f32 => "Low Listener Increase"                => [0.0, 0.005],
    high_listener_dec:       f32 => "High Listener Decrease"               => [0.0, 0.02],
    high_listener_dec_every: f32 => "High Listener Decrease Per Listeners" => [1.0, 100.0],
);

create_config_struct!(UnskewedAverage,
    reset_pcnt:      f32 => "Reset To Average Percentage"  => [0.0, 0.15],
    adjust_pcnt:     f32 => "Adjust to Average Percentage" => [0.0, 0.0075],
    spikes_required: u32 => "Spikes Required"              => 1,
    jump_required:   f32 => "Jump Required To Set"         => [1.1, 4.0],
);

create_config_enum!(FeedIdent,
    Name(String)   => self,
    ID(u32)        => self,
    County(String) => self,
    State(u32)     => "State ID",
);

impl FeedIdent {
    /// Returns true if the FeedIdent matches the corresponding feed data.
    pub fn matches_feed(&self, feed: &Feed) -> bool {
        match self {
            FeedIdent::Name(name) => *name == feed.name,
            FeedIdent::ID(id) => *id == feed.id,
            FeedIdent::County(c) => *c == feed.county,
            FeedIdent::State(id) => *id == feed.location.state_id,
        }
    }
}

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
    /// Returns the spike values for the current day, if it exists in the specified array.
    pub fn get_for_today(weekday_spikes: &[WeekdaySpike]) -> Option<&Spike> {
        use self::WeekdaySpike::*;
        use chrono::Weekday::*;

        let weekday = Local::today().weekday();

        for ws in weekday_spikes {
            match (weekday, ws) {
                (Mon, Monday(s))
                | (Tue, Tuesday(s))
                | (Wed, Wednesday(s))
                | (Thu, Thursday(s))
                | (Fri, Friday(s))
                | (Sat, Saturday(s))
                | (Sun, Sunday(s)) => return Some(s),
                _ => (),
            }
        }

        None
    }
}

create_config_struct!(FeedSetting,
    ident:          FeedIdent         => self                        => fail,
    spike:          Spike             => "Spike Percentages"         => default,
    weekday_spikes: Vec<WeekdaySpike> => "Weekday Spike Percentages" => all,
);

create_config_struct!(Misc,
	update_time:       f32              => "Update Time"              => [5.0, 6.0],
	minimum_listeners: u32              => "Minimum Listeners"        => 15,
	state_feeds_id:    Option<u32>      => "State Feeds ID"           => None,
    max_feeds:         u32              => "Maximum Feeds To Display" => 10,
    show_alert_feeds:  bool             => "Show Alert Feeds"         => true,
    max_times_to_show_feed: Option<u32> => "Max Times To Show Feed"   => None,
);

create_config_enum!(SortType,
    Listeners => self,
    Jump      => self,
);

create_config_enum!(SortOrder,
    Ascending  => self,
    Descending => self,
);

create_config_struct!(Sorting,
    sort_type:  SortType  => "Sort By"    => { SortType::Listeners },
    sort_order: SortOrder => "Sort Order" => { SortOrder::Descending },
);

trait ParseYaml: Sized + Default {
    fn from(doc: &Yaml) -> Option<Self>;

    fn from_or_default(doc: &Yaml) -> Self {
        ParseYaml::from(doc).unwrap_or_default()
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
                use yaml_rust::Yaml::*;
                match doc {
                    Integer(num) => Some(*num as $t),
                    Real(string) => string.parse().ok(),
                    _ => None,
                }
            }
        }
        )+
    }
}

impl_parseyaml_for_numeric!(u8 u32 f32);

impl ParseYaml for String {
    fn from(doc: &Yaml) -> Option<String> {
        if let Yaml::String(s) = doc {
            Some(s.clone())
        } else {
            None
        }
    }
}

impl ParseYaml for bool {
    fn from(doc: &Yaml) -> Option<bool> {
        if let Yaml::Boolean(value) = doc {
            Some(*value)
        } else {
            None
        }
    }
}
