extern crate yaml_rust;

use std::error::Error;
use std::path::Path;
use util;
use self::yaml_rust::{YamlLoader, Yaml};

#[macro_use] mod macros;

create_config_enum!(Blacklist,
    Name(String) => "Name",
    Id(i32)      => "Id",
);

create_config_arr!(FeedPercentage,
    name:  String => "Name",
    spike: f32    => "Spike",
);

create_config_section!(Global,
	spike:                   f32        => "Spike Percentage"                     => 0.25,
	low_listener_increase:   f32        => "Low Listener Increase Percentage"     => [0.0, 0.005],
	high_listener_dec:       f32        => "High Listener Decrease Percentage"    => [0.0, 0.02],
	high_listener_dec_every: f32        => "High Listener Decrease Per Listeners" => [1.0, 100.0],
	update_time:             f32        => "Update Time"                          => [5.0, 6.0],
	minimum_listeners:       u32        => "Minimum Listeners"                    => 15,
	state_feeds_id:          Option<u8> => "State Feeds ID"                       => None,
);

create_config_section!(UnskewedAverage,
    reset_pcnt:      f32 => "Reset To Average Percentage"  => [0.0, 0.15],
    adjust_pcnt:     f32 => "Adjust to Average Percentage" => [0.0, 0.01],
    spikes_required: u8  => "Listener Spikes Required"     => 1,
);

#[derive(Debug)]
pub struct Config {
    pub global:           Global,
    pub unskewed_avg:     UnskewedAverage,
    pub feed_percentages: Vec<FeedPercentage>,
    pub blacklist:        Vec<Blacklist>,
}

pub fn load_from_file(path: &Path) -> Result<Config, Box<Error>> {
    let doc = YamlLoader::load_from_str(&util::read_file(path)?)?;
    let doc = &doc[0]; // We don't care about multiple documents

    Ok(Config {
        global:           Global::new(&doc["Global"]),
        unskewed_avg:     UnskewedAverage::new(&doc["Unskewed Average"]),
        feed_percentages: FeedPercentage::parse(&doc["Feed Percentages"]),
        blacklist:        Blacklist::parse(&doc["Blacklist"]).unwrap_or(Vec::new()),
    })
}