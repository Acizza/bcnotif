extern crate yaml_rust;

use std::error::Error;
use std::path::Path;
use util;
use self::yaml_rust::{YamlLoader, Yaml};

#[macro_use] mod macros;

create_config_enum!(FeedIdent,
    Name(String) => "Name",
    ID(u32)      => "ID",
);

create_config_arr!(FeedSetting,
    id:   u32 => "ID",
    jump: f32 => "Jump Required",
);

create_config_section!(Spike,
    jump:                    f32 => "Jump Required"                        => 0.25,
    low_listener_increase:   f32 => "Low Listener Increase"                => [0.0, 0.005],
    high_listener_dec:       f32 => "High Listener Decrease"               => [0.0, 0.02],
    high_listener_dec_every: f32 => "High Listener Decrease Per Listeners" => [1.0, 100.0],
);

create_config_section!(UnskewedAverage,
    reset_pcnt:      f32 => "Reset To Average Percentage"  => [0.0, 0.15],
    adjust_pcnt:     f32 => "Adjust to Average Percentage" => [0.0, 0.01],
    spikes_required: u8  => "Listener Spikes Required"     => 1,
);

create_config_section!(Misc,
	update_time:       f32        => "Update Time"       => [5.0, 6.0],
	minimum_listeners: u32        => "Minimum Listeners" => 15,
	state_feeds_id:    Option<u8> => "State Feeds ID"    => None,
);

#[derive(Debug)]
pub struct Config {
    pub spike:         Spike,
    pub unskewed_avg:  UnskewedAverage,
    pub misc:          Misc,
    pub feed_settings: Vec<FeedSetting>,
    pub blacklist:     Vec<FeedIdent>,
    pub whitelist:     Vec<FeedIdent>,
}

pub fn load_from_file(path: &Path) -> Result<Config, Box<Error>> {
    let doc = YamlLoader::load_from_str(&util::read_file(path)?)?;
    let doc = &doc[0]; // We don't care about multiple documents

    Ok(Config {
        spike:         Spike::new(&doc["Spike Percentages"]),
        unskewed_avg:  UnskewedAverage::new(&doc["Unskewed Average"]),
        misc:          Misc::new(&doc["Misc"]),
        feed_settings: FeedSetting::parse(&doc["Feed Settings"]),
        blacklist:     FeedIdent::parse(&doc["Blacklist"]).unwrap_or(Vec::new()),
        whitelist:     FeedIdent::parse(&doc["Whitelist"]).unwrap_or(Vec::new()),
    })
}