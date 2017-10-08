extern crate csv;

use config::Config;
use chrono::{Utc, Timelike};
use feed::Feed;
use std::collections::HashMap;
use std::path::PathBuf;
use ::math;

error_chain! {
    foreign_links {
        Csv(csv::Error);
        ParseInt(::std::num::ParseIntError);
        ParseFloat(::std::num::ParseFloatError);
        Utf8Error(::std::string::FromUtf8Error);
        Io(::std::io::Error);
    }

    errors {
        TooFewRows {
            display("csv file contains record with too few rows")
        }
    }
}

pub struct AverageData {
    pub path: PathBuf,
    pub data: HashMap<u32, ListenerStats>,
}

impl AverageData {
    pub fn new(path: PathBuf) -> AverageData {
        AverageData {
            path,
            data: HashMap::new(),
        }
    }

    pub fn load(&mut self) -> Result<()> {
        let hour = Utc::now().hour() as usize;
        let mut rdr = csv::Reader::from_path(&self.path)?;

        for result in rdr.records() {
            let record = result?;
            
            if record.len() < 1 + ListenerStats::HOURLY_SIZE {
                bail!(ErrorKind::TooFewRows);
            }

            let id = record[0].parse::<u32>()?;
            let mut averages = [0.0; ListenerStats::HOURLY_SIZE];

            for i in 0..ListenerStats::HOURLY_SIZE {
                // Use an offset of 1 to avoid capturing the feed ID field
                averages[i] = record[1 + i].parse::<f32>()?;
            }

            let listeners = averages[hour] as i32;
            self.data.insert(id, ListenerStats::with_data(listeners, averages));
        }

        Ok(())
    }

    pub fn save(&self) -> Result<()> {
        let mut wtr = csv::Writer::from_path(&self.path)?;
        let mut fields = Vec::with_capacity(1 + ListenerStats::HOURLY_SIZE);

        for (id, stats) in &self.data {
            fields.push(id.to_string());

            for average in stats.average_hourly.iter() {
                fields.push(average.round().to_string());
            }

            wtr.write_record(&fields)?;
            fields.clear();
        }

        wtr.flush()?;
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct Average {
    pub current: f32,
    pub last: f32,
    pub data: Vec<i32>,
    index: usize,
    populated: usize,
}

impl Average {
    pub fn new(sample_size: usize) -> Average {
        Average {
            current: 0.0,
            last: 0.0,
            data: vec![0; sample_size],
            index: 0,
            populated: 0,
        }
    }

    pub fn with_value(sample_size: usize, value: i32) -> Average {
        let mut average = Average::new(sample_size);
        average.add_sample(value);

        average
    }

    pub fn add_sample(&mut self, value: i32) {
        if self.index >= self.data.len() {
            self.index = 0;
        }

        self.data[self.index] = value;
        self.index += 1;

        if self.populated < self.index {
            self.populated += 1;
        }

        self.last = self.current;

        self.current = self.data
            .iter()
            .take(self.populated)
            .sum::<i32>() as f32 / self.populated as f32;
    }
}

#[derive(Debug, Clone)]
pub struct ListenerStats {
    /// Represents the average number of listeners
    pub average: Average,
    /// Represents the average number of listeners before a consistent spike occured
    pub unskewed_average: Option<f32>,
    /// Contains the average number of listeners for any given hour
    pub average_hourly: [f32; ListenerStats::HOURLY_SIZE],
    /// Indicates whether or not the listner count has spiked since the last update
    pub has_spiked: bool,
    /// Represents the number of times the feed has spiked consecutively
    pub spike_count: u32,
}

impl ListenerStats {
    const AVERAGE_SIZE: usize = 5;
    const HOURLY_SIZE: usize = 24;

    pub fn new() -> ListenerStats {
        ListenerStats {
            average: Average::new(ListenerStats::AVERAGE_SIZE),
            unskewed_average: None,
            average_hourly: [0.0; ListenerStats::HOURLY_SIZE],
            has_spiked: false,
            spike_count: 0,
        }
    }

    pub fn with_data(listeners: i32, hourly: [f32; ListenerStats::HOURLY_SIZE]) -> ListenerStats {
        ListenerStats {
            average: Average::with_value(ListenerStats::AVERAGE_SIZE, listeners),
            unskewed_average: None,
            average_hourly: hourly,
            has_spiked: false,
            spike_count: 0,
        }
    }

    pub fn update(&mut self, hour: usize, feed: &Feed, config: &Config) {
        self.has_spiked = self.is_spiking(feed, config);

        self.spike_count = if self.has_spiked {
            self.spike_count + 1
        } else {
            0
        };

        self.update_listeners(hour, feed.listeners);
        self.update_unskewed_average(feed.listeners, config);
    }

    fn is_spiking(&self, feed: &Feed, config: &Config) -> bool {
        if self.average.current == 0.0 {
            return false;
        }

        let spike = config.get_feed_spike(&feed);
        let listeners = feed.listeners as f32;
        
        let threshold = if listeners < 50.0 {
            spike.jump + (50.0 - listeners) * spike.low_listener_increase
        } else {
            let delta = self.get_average_delta(feed.listeners);
            let rise_amount = delta / spike.high_listener_dec_every * spike.high_listener_dec;

            spike.jump - rise_amount.min(spike.jump - 0.01)
        };

        listeners - self.average.current >= listeners * threshold
    }

    fn update_listeners(&mut self, hour: usize, listeners: u32) {
        self.average.add_sample(listeners as i32);
        self.average_hourly[hour] = self.average.current;
    }

    fn update_unskewed_average(&mut self, listeners: u32, config: &Config) {
        if let Some(unskewed) = self.unskewed_average {
            if self.average.current - unskewed < unskewed * config.unskewed_avg.reset_pcnt {
                self.unskewed_average = None;
            } else {
                self.unskewed_average = Some(math::lerp(
                    unskewed,
                    self.average.current,
                    config.unskewed_avg.adjust_pcnt
                ));
            }
        } else if self.has_spiked && self.average.last > 0.0 {
            let has_large_jump =
                listeners as f32 - self.average.current >
                self.average.last * config.unskewed_avg.jump_required;

            if self.spike_count > 1 || has_large_jump {
                self.unskewed_average = Some(self.average.last);
            }
        }
    }

    pub fn get_average_delta(&self, listeners: u32) -> f32 {
        listeners as f32 - self.unskewed_average.unwrap_or(self.average.current)
    }
}