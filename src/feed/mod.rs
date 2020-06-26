pub mod stats;

mod scrape;

use crate::config::Config;
use anyhow::{anyhow, Context, Result};
use notify_rust::Notification;
use serde::de::Visitor;
use serde::{Deserialize, Deserializer};
use stats::ListenerStats;
use std::borrow::Cow;
use std::cmp::{self, Eq, Ord};
use std::fmt;
use std::result;
use std::str::FromStr;
use std::time::Duration;
use strum_macros::EnumString;

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

        if let Some(loc) = config.misc.location {
            let loc_feeds =
                Self::scrape_source(Source::Location(loc), config.misc.minimum_listeners)?;

            feeds.extend(loc_feeds);
        }

        feeds.sort_unstable();
        feeds.dedup();

        Ok(feeds)
    }

    fn scrape_source(source: Source, min_listeners: u32) -> Result<Vec<Self>> {
        let resp = attohttpc::get(source.url().as_ref())
            .timeout(Duration::from_secs(15))
            .send()
            .context("http request failed")?;

        if !resp.is_success() {
            return Err(anyhow!(
                "received bad status from Broadcastify: {}",
                resp.status()
            ));
        }

        let body = resp.text().context("failed to read text from response")?;

        match source {
            Source::Top50 => {
                scrape::scrape_top(&body, min_listeners).context("failed to parse top 50 feeds")
            }
            Source::Location(location) => scrape::scrape_location(&body, min_listeners, location)
                .with_context(|| anyhow!("failed to parse feeds for {}", location.abbrev())),
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

pub enum Source {
    Top50,
    Location(Location),
}

impl Source {
    pub fn url(&self) -> Cow<str> {
        match self {
            Self::Top50 => "https://www.broadcastify.com/listen/top".into(),
            Self::Location(loc) => {
                format!("https://www.broadcastify.com/listen/stid/{}", loc.id()).into()
            }
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

        let alert = match &self.feed.alert {
            Some(alert) => Cow::Owned(format!("\nalert: {}", alert)),
            None => Cow::Borrowed(""),
        };

        let body = format!(
            "{abbrev} | {name}\n{listeners} (^{jump}){alert}",
            abbrev = self.feed.location.abbrev(),
            name = self.feed.name,
            listeners = self.feed.listeners,
            jump = self.jump as i32,
            alert = alert,
        );

        Notification::new()
            .summary(&title)
            .body(&body)
            .show()
            .map_err(|err| anyhow!("failed to create notification: {}", err))
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

/// List of all states / provinces / territories on Broadcastify that have a significant feed presence or airport feeds.
/// Every location can be mapped to its state ID as it appears on Broadcastify.
///
/// Airport feeds are used as a factor for being on this list because there's a chance of a major event occuring on them, at least more so than just amateur radio stations.
#[derive(Copy, Clone, Debug, EnumString, Eq, FromPrimitive, Hash, PartialEq)]
#[strum(serialize_all = "kebab_case")]
#[repr(u32)]
pub enum Location {
    // United States
    UsAlabama = 1,
    UsAlaska,
    UsArizona = 4,
    UsArkansas,
    UsCalifornia,
    UsColorado = 8,
    UsConnecticut,
    UsDelaware,
    UsDistrictOfColumbia,
    UsFlorida,
    UsGeorgia,
    UsHawaii = 15,
    UsIdaho,
    UsIllinois,
    UsIndiana,
    UsIowa,
    UsKansas,
    UsKentucky,
    UsLouisiana,
    UsMaine,
    UsMaryland,
    UsMassachusetts,
    UsMichigan,
    UsMinnesota,
    UsMississippi,
    UsMissouri,
    UsMontana,
    UsNebraska,
    UsNevada,
    UsNewHampshire,
    UsNewJersey,
    UsNewMexico,
    UsNewYork,
    UsNorthCarolina,
    UsNorthDakota,
    UsOhio,
    UsOklahoma,
    UsOregon,
    UsPennsylvania,
    UsRhodeIsland = 44,
    UsSouthCarolina,
    UsSouthDakota,
    UsTennessee,
    UsTexas,
    UsUtah,
    UsVermont,
    UsVirginia,
    UsWashington = 53,
    UsWestVirginia,
    UsWisconsin,
    UsWyoming,
    // Canada
    CaAlberta = 101,
    CaBritishColumbia,
    CaManitoba,
    CaNewBrunswick,
    CaNewfoundland,
    CaNorthwestTerritories,
    CaNovaScotia,
    CaNunavut,
    CaOntario,
    CaPrinceEdwardIsland,
    CaQuebec,
    CaSaskatchewan,
    CaYukon,
    // Austrailia
    AuWesternAustrailia = 151,
    AuVictoria,
    AuSouthAustrailia,
    AuNorthernTerritory,
    AuQueensland,
    AuTasmania,
    AuNewSouthWales,
    AuAustralianCapitalTerritory,
    // Netherlands
    NlCountrywide = 223,
    NlDrenthe = 688,
    NlFlevoland,
    NlFriesland,
    NlGelderland,
    NlGroningen,
    NlLimburg,
    NlNoordBrabant,
    NlNoordHolland,
    NlOverijssel,
    NlUtrecht,
    NlZeeland,
    NlZuidHolland,
    // Malaysia
    MyCountrywide = 231,
    // Luxembourg
    LuCountrywide = 252,
    // Brazil
    BrCountrywide = 345,
    // Chile
    ClValparaiso = 714,
    ClBiobio = 717,
    ClAraucania,
    ClSantiago = 723,
}

impl Location {
    const LISTED_COUNTRIES: [&'static str; 8] = [
        "Austrailia",
        "Brazil",
        "Canada",
        "Chile",
        "Luxembourg",
        "Malaysia",
        "Netherlands",
        "United States",
    ];

    #[inline(always)]
    pub fn id(self) -> u32 {
        self as u32
    }

    pub fn abbrev(self) -> &'static str {
        match self {
            // United States
            Self::UsAlabama => "US-AL",
            Self::UsAlaska => "US-AK",
            Self::UsArizona => "US-AZ",
            Self::UsArkansas => "US-AR",
            Self::UsCalifornia => "US-CA",
            Self::UsColorado => "US-CO",
            Self::UsConnecticut => "US-CT",
            Self::UsDelaware => "US-DE",
            Self::UsDistrictOfColumbia => "US-DC",
            Self::UsFlorida => "US-FL",
            Self::UsGeorgia => "US-GA",
            Self::UsHawaii => "US-HI",
            Self::UsIdaho => "US-ID",
            Self::UsIllinois => "US-IL",
            Self::UsIndiana => "US-IN",
            Self::UsIowa => "US-IA",
            Self::UsKansas => "US-KS",
            Self::UsKentucky => "US-KY",
            Self::UsLouisiana => "US-LA",
            Self::UsMaine => "US-ME",
            Self::UsMaryland => "US-MD",
            Self::UsMassachusetts => "US-MA",
            Self::UsMichigan => "US-MI",
            Self::UsMinnesota => "US-MN",
            Self::UsMississippi => "US-MS",
            Self::UsMissouri => "US-MO",
            Self::UsMontana => "US-MT",
            Self::UsNebraska => "US-NE",
            Self::UsNevada => "US-NV",
            Self::UsNewHampshire => "US-NH",
            Self::UsNewJersey => "US-NJ",
            Self::UsNewMexico => "US-NM",
            Self::UsNewYork => "US-NY",
            Self::UsNorthCarolina => "US-NC",
            Self::UsNorthDakota => "US-ND",
            Self::UsOhio => "US-OH",
            Self::UsOklahoma => "US-OK",
            Self::UsOregon => "US-OR",
            Self::UsPennsylvania => "US-PA",
            Self::UsRhodeIsland => "US-RI",
            Self::UsSouthCarolina => "US-SC",
            Self::UsSouthDakota => "US-SD",
            Self::UsTennessee => "US-TN",
            Self::UsTexas => "US-TX",
            Self::UsUtah => "US-UT",
            Self::UsVermont => "US-VT",
            Self::UsVirginia => "US-VA",
            Self::UsWashington => "US-WA",
            Self::UsWestVirginia => "US-WV",
            Self::UsWisconsin => "US-WI",
            Self::UsWyoming => "US-WY",
            // Canada
            Self::CaAlberta => "CA-AB",
            Self::CaBritishColumbia => "CA-BC",
            Self::CaManitoba => "CA-MB",
            Self::CaNewBrunswick => "CA-NB",
            Self::CaNewfoundland => "CA-NL",
            Self::CaNorthwestTerritories => "CA-NT",
            Self::CaNovaScotia => "CA-NS",
            Self::CaNunavut => "CA-NU",
            Self::CaOntario => "CA-ON",
            Self::CaPrinceEdwardIsland => "CA-PE",
            Self::CaQuebec => "CA-QC",
            Self::CaSaskatchewan => "CA-SK",
            Self::CaYukon => "CA-YT",
            // Austrailia
            Self::AuWesternAustrailia => "AU-WA",
            Self::AuVictoria => "AU-VIC",
            Self::AuSouthAustrailia => "AU-SA",
            Self::AuNorthernTerritory => "AU-NT",
            Self::AuQueensland => "AU-QLD",
            Self::AuTasmania => "AU-TAS",
            Self::AuNewSouthWales => "AU-NSW",
            Self::AuAustralianCapitalTerritory => "AU-ACT",
            // Netherlands
            Self::NlCountrywide => "NL-ALL",
            Self::NlDrenthe => "NL-DR",
            Self::NlFlevoland => "NL-FL",
            Self::NlFriesland => "NL-FR",
            Self::NlGelderland => "NL-GE",
            Self::NlGroningen => "NL-GR",
            Self::NlLimburg => "NL-LI",
            Self::NlNoordBrabant => "NL-NB",
            Self::NlNoordHolland => "NL-NH",
            Self::NlOverijssel => "NL-OV",
            Self::NlUtrecht => "NL-UT",
            Self::NlZeeland => "NL-ZE",
            Self::NlZuidHolland => "NL-ZH",
            // Malaysia
            Self::MyCountrywide => "MY-ALL",
            // Luxembourg
            Self::LuCountrywide => "LU-ALL",
            // Brazil
            Self::BrCountrywide => "BR-ALL",
            // Chile
            Self::ClValparaiso => "CL-VL",
            Self::ClBiobio => "CL-BI",
            Self::ClAraucania => "CL-AR",
            Self::ClSantiago => "CL-RM",
        }
    }
}

impl<'de> Deserialize<'de> for Location {
    fn deserialize<D>(deserializer: D) -> result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct LocationVisitor;

        impl<'de> Visitor<'de> for LocationVisitor {
            type Value = Location;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                let country_list = Location::LISTED_COUNTRIES.join("\n\t");
                formatter.write_str(&format!("a state / province / territory located in one of the following countries:\n\t{}\n", country_list))
            }

            fn visit_str<E>(self, value: &str) -> result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                use serde::de::{self, Unexpected};

                Location::from_str(value)
                    .map_err(|_| de::Error::invalid_value(Unexpected::Str(value), &self))
            }
        }

        deserializer.deserialize_str(LocationVisitor)
    }
}
