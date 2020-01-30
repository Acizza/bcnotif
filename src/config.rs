use crate::err::{self, Result};
use crate::feed::Feed;
use crate::path::FilePath;
use chrono::Weekday;
use serde::de::Visitor;
use serde::{Deserialize, Deserializer};
use serde_derive::Deserialize;
use snafu::ResultExt;
use std::borrow::Cow;
use std::collections::HashMap;
use std::fmt;
use std::fs;
use std::path::PathBuf;
use std::result;

#[derive(Debug, Default, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub weekday: HashMap<Weekday, FeedOptionMap>,
    #[serde(default)]
    pub feed: FeedOptionMap,
    #[serde(default)]
    pub misc: MiscOptions,
    #[serde(default)]
    pub sorting: SortOptions,
    #[serde(default)]
    pub filters: FilterOptions,
}

impl Config {
    pub fn load_or_new() -> Result<Self> {
        match Self::load() {
            Ok(cfg) => Ok(cfg),
            Err(err) if err.is_file_nonexistant() => Ok(Self::default()),
            err => err,
        }
    }

    pub fn load() -> Result<Self> {
        let path = Self::validated_path()?;
        let contents = fs::read_to_string(&path).context(err::FileIO { path })?;
        let config = toml::from_str(&contents).context(err::TOMLDecode { name: "config" })?;
        Ok(config)
    }

    pub fn validated_path() -> Result<PathBuf> {
        let mut path = FilePath::Config.validated_dir_path()?;
        path.push("config.toml");
        Ok(path)
    }

    pub fn options_for_feed<'a>(&'a self, feed: &Feed, weekday: Weekday) -> Cow<'a, FeedOptions> {
        let selector = match self.weekday.get(&weekday) {
            Some(weekday_opts) => weekday_opts,
            None => &self.feed,
        };

        selector
            .iter()
            .find(|(sel, _)| sel.matches_feed(feed))
            .map_or_else(|| FeedOptions::default().into(), |(_, value)| value.into())
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct FeedOptions {
    #[serde(default = "FeedOptions::jump_required_default")]
    pub jump_required: Percentage,
    #[serde(
        rename = "jump_required_set_unskewed",
        default = "FeedOptions::jump_required_unskewed_default"
    )]
    pub jump_required_unskewed: Percentage,
}

impl FeedOptions {
    fn jump_required_default() -> Percentage {
        Percentage::new(40.0)
    }

    fn jump_required_unskewed_default() -> Percentage {
        Percentage::new(400.0)
    }
}

impl Default for FeedOptions {
    fn default() -> Self {
        Self {
            jump_required: Self::jump_required_default(),
            jump_required_unskewed: Self::jump_required_unskewed_default(),
        }
    }
}

impl<'a> Into<Cow<'a, Self>> for FeedOptions {
    fn into(self) -> Cow<'a, Self> {
        Cow::Owned(self)
    }
}

impl<'a> Into<Cow<'a, FeedOptions>> for &'a FeedOptions {
    fn into(self) -> Cow<'a, FeedOptions> {
        Cow::Borrowed(self)
    }
}

pub type FeedOptionMap = HashMap<FeedSelector, FeedOptions>;

#[derive(Debug, Deserialize)]
pub struct MiscOptions {
    #[serde(default = "MiscOptions::update_time_mins_default")]
    pub update_time_mins: f32,
    #[serde(default = "MiscOptions::minimum_listeners_default")]
    pub minimum_listeners: u32,
    pub state_id: Option<u32>,
    #[serde(default = "MiscOptions::show_max_default")]
    pub show_max: u32,
    pub show_max_times: Option<u32>,
    #[serde(default = "MiscOptions::show_alert_feeds_default")]
    pub show_alert_feeds: bool,
}

impl MiscOptions {
    const fn update_time_mins_default() -> f32 {
        6.0
    }

    const fn minimum_listeners_default() -> u32 {
        15
    }

    const fn show_max_default() -> u32 {
        10
    }

    const fn show_alert_feeds_default() -> bool {
        true
    }
}

impl Default for MiscOptions {
    fn default() -> Self {
        Self {
            update_time_mins: Self::update_time_mins_default(),
            minimum_listeners: Self::minimum_listeners_default(),
            state_id: None,
            show_max: Self::show_max_default(),
            show_max_times: None,
            show_alert_feeds: Self::show_alert_feeds_default(),
        }
    }
}

#[derive(Debug, Default, Deserialize)]
pub struct SortOptions {
    #[serde(default)]
    pub value: SortType,
    #[serde(default)]
    pub order: SortOrder,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SortType {
    Jump,
    Listeners,
}

impl Default for SortType {
    fn default() -> Self {
        Self::Jump
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SortOrder {
    Ascending,
    Descending,
}

impl Default for SortOrder {
    fn default() -> Self {
        Self::Descending
    }
}

#[derive(Debug, Default, Deserialize)]
pub struct FilterOptions {
    #[serde(default)]
    pub blacklist: Vec<FeedSelector>,
    #[serde(default)]
    pub whitelist: Vec<FeedSelector>,
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub enum FeedSelector {
    Global,
    ID(u32),
    County(String),
    State(u32),
}

impl FeedSelector {
    fn from_str<S>(value: S) -> Option<Self>
    where
        S: AsRef<str>,
    {
        match value.as_ref() {
            "global" => Some(Self::Global),
            selector => {
                if !selector.ends_with(')') {
                    return None;
                }

                let opening_bracket = selector.find('(')?;
                let sel_name = selector[..opening_bracket].to_ascii_lowercase();
                let sel_value = &selector[opening_bracket + 1..selector.len() - 1];

                match sel_name.as_ref() {
                    "id" => sel_value.parse().ok().map(Self::ID),
                    "county" => Some(Self::County(sel_value.into())),
                    "state" => sel_value.parse().ok().map(Self::State),
                    _ => None,
                }
            }
        }
    }

    pub fn matches_feed(&self, feed: &Feed) -> bool {
        match self {
            Self::Global => true,
            Self::ID(id) => *id == feed.id,
            Self::County(county) => county.eq_ignore_ascii_case(&feed.county),
            Self::State(state) => *state == feed.location.state_id,
        }
    }
}

impl Default for FeedSelector {
    fn default() -> Self {
        Self::Global
    }
}

impl<'de> Deserialize<'de> for FeedSelector {
    fn deserialize<D>(deserializer: D) -> result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct FeedSelectorVisitor;

        impl<'de> Visitor<'de> for FeedSelectorVisitor {
            type Value = FeedSelector;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a feed selector (global, id(id), county(name), state(id))")
            }

            fn visit_str<E>(self, value: &str) -> result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                use serde::de::{self, Unexpected};

                FeedSelector::from_str(value)
                    .ok_or_else(|| de::Error::invalid_value(Unexpected::Str(value), &self))
            }
        }

        deserializer.deserialize_str(FeedSelectorVisitor)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Percentage(f32);

impl Percentage {
    pub fn new(pcnt: f32) -> Self {
        Self(pcnt / 100.0)
    }

    pub fn as_mult(self) -> f32 {
        self.0
    }
}

impl<'de> Deserialize<'de> for Percentage {
    fn deserialize<D>(deserializer: D) -> result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct PercentageVisitor;

        impl<'de> Visitor<'de> for PercentageVisitor {
            type Value = f32;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a percentage between 0 and 100")
            }

            fn visit_f32<E>(self, value: f32) -> result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(value / 100.0)
            }

            fn visit_f64<E>(self, value: f64) -> result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(value as f32 / 100.0)
            }

            fn visit_i32<E>(self, value: i32) -> result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(value as f32 / 100.0)
            }

            fn visit_i64<E>(self, value: i64) -> result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(value as f32 / 100.0)
            }
        }

        let raw_pcnt = deserializer.deserialize_f32(PercentageVisitor)?;
        Ok(Self(raw_pcnt))
    }
}
