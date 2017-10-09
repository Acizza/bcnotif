extern crate yaml_rust;

#[macro_use] mod generation;

use chrono::{Local, Datelike};
use feed::Feed;
use self::yaml_rust::{YamlLoader, Yaml};
use std::path::Path;

error_chain! {
    links {
        Util(::util::Error, ::util::ErrorKind);
    }

    foreign_links {
        Yaml(yaml_rust::ScanError);
    }
}

create_config_struct!(Spike,
    jump:                    f32 => "Jump Required"                        => 0.3,
    low_listener_increase:   f32 => "Low Listener Increase"                => [0.0, 0.005],
    high_listener_dec:       f32 => "High Listener Decrease"               => [0.0, 0.02],
    high_listener_dec_every: f32 => "High Listener Decrease Per Listeners" => [1.0, 100.0],
);

create_config_struct!(UnskewedAverage,
    reset_pcnt:      f32 => "Reset To Average Percentage"  => [0.0, 0.15],
    adjust_pcnt:     f32 => "Adjust to Average Percentage" => [0.0, 0.0075],
    spikes_required: u8  => "Listener Spikes Required"     => 1,
    jump_required:   f32 => "Listener Jump Required"       => [1.1, 4.0],
);

create_config_enum!(FeedIdent,
    Name(String)   => self,
    ID(u32)        => self,
    County(String) => self,
    State(u32)     => "State ID",
);

impl FeedIdent {
    pub fn matches_feed(&self, feed: &Feed) -> bool {
        match *self {
            FeedIdent::Name(ref name) => *name == feed.name,
            FeedIdent::ID(id)         => id == feed.id,
            FeedIdent::County(ref c)  => *c == feed.county,
            FeedIdent::State(id)      => id == feed.state_id,
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

create_config_struct!(FeedSetting,
    ident:          FeedIdent         => self                        => fail,
    spike:          Spike             => "Spike Percentages"         => default,
    weekday_spikes: Vec<WeekdaySpike> => "Weekday Spike Percentages" => all,
);

create_config_struct!(Misc,
	update_time:       f32         => "Update Time"       => [5.0, 6.0],
	minimum_listeners: u32         => "Minimum Listeners" => 15,
	state_feeds_id:    Option<u32> => "State Feeds ID"    => None,
    sort_order:        SortOrder   => "Feed Sort Order"   => (SortOrder::Descending),
);

create_config_enum!(SortOrder,
    Ascending  => self,
    Descending => self,
);

macro_rules! gen_base_parse_stmt {
    (optional, $category:expr, $doc:ident) => (ParseYaml::from(&$doc[$category]));
    (default,  $category:expr, $doc:ident) => (ParseYaml::from_or_default(&$doc[$category]));
    (all,      $category:expr, $doc:ident) => (ParseYaml::all(&$doc[$category]));
}

macro_rules! gen_base_config {
    ($name:ident, $($field:ident: $type:ty => $parse_type:ident => $category:expr,)+) => {
        #[derive(Debug, Default)]
        pub struct $name {
            $(pub $field: $type,)+
        }

        impl $name {
            pub fn from_file(path: &Path) -> Result<$name> {
                let file = ::util::read_file(path)?;

                if file.len() == 0 {
                    return Ok(Config::default())
                }

                let doc = YamlLoader::load_from_str(&::util::read_file(path)?)?;
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
    blacklist:      Vec<FeedIdent>    => all     => "Blacklist",
    whitelist:      Vec<FeedIdent>    => all     => "Whitelist",
);

impl Config {
    pub fn get_feed_spike(&self, feed: &Feed) -> &Spike {
        // Find any settings for the specified feed
        let feed_setting = self.feed_settings
            .iter()
            .find(|s| s.ident.matches_feed(&feed));

        match feed_setting {
            Some(setting) => {
                WeekdaySpike::get_for_today(&setting.weekday_spikes)
                    .unwrap_or(&setting.spike)
            },
            None => {
                WeekdaySpike::get_for_today(&self.weekday_spikes)
                    .unwrap_or(&self.global_spike)
            }
        }
    }
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