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

type FeedID = u32;

/// An interface to save and load ListenerStats data.
pub struct AverageData {
    /// The path to the file to save and load data from.
    pub path: PathBuf,
    /// The data to save and load.
    pub data: HashMap<FeedID, ListenerStats>,
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

            let id = record[0].parse()?;
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

/// Represents an average set of data that wraps around its specified sample size.
#[derive(Debug, Clone)]
pub struct Average {
    /// The current average. It is updated after calling to self.add_sample().
    pub current: f32,
    /// The current average before the last call to self.add_sample().
    pub last: f32,
    /// The raw data that is used to calculate the current and last average.
    pub data: Vec<i32>,
    /// The current data index.
    index: usize,
    /// This keeps track of how many samples have been added since the struct
    /// was created, up until it reaches the specified sample size.
    /// It prevents data values that haven't been added from being averaged.
    populated: usize,
}

impl Average {
    pub fn new(sample_size: usize) -> Average {
        Average {
            current:   0.0,
            last:      0.0,
            data:      vec![0; sample_size],
            index:     0,
            populated: 0,
        }
    }

    /// Creates a new Average with one initial sample.
    pub fn with_value(sample_size: usize, value: i32) -> Average {
        let mut average = Average::new(sample_size);
        average.add_sample(value);

        average
    }

    /// Adds a new sample to the data and calculates the new average.
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

/// Represents general statistical data for feeds.
#[derive(Debug, Clone)]
pub struct ListenerStats {
    /// Represents the average number of listeners.
    pub average: Average,
    /// Represents the average number of listeners before a consistent spike occured.
    pub unskewed_average: Option<f32>,
    /// Contains the average number of listeners for any given hour.
    pub average_hourly: [f32; ListenerStats::HOURLY_SIZE],
    /// Indicates whether or not the listner count has spiked since the last update.
    pub has_spiked: bool,
    /// Represents the number of times the feed has spiked consecutively.
    pub spike_count: u32,
}

impl ListenerStats {
    const AVERAGE_SIZE: usize = 5;
    const HOURLY_SIZE:  usize = 24;

    pub fn new() -> ListenerStats {
        ListenerStats {
            average:          Average::new(ListenerStats::AVERAGE_SIZE),
            unskewed_average: None,
            average_hourly:   [0.0; ListenerStats::HOURLY_SIZE],
            has_spiked:       false,
            spike_count:      0,
        }
    }

    /// Creates a new ListenerStats struct with existing listener and hourly listener data.
    pub fn with_data(listeners: i32, hourly: [f32; ListenerStats::HOURLY_SIZE]) -> ListenerStats {
        ListenerStats {
            average:          Average::with_value(ListenerStats::AVERAGE_SIZE, listeners),
            unskewed_average: None,
            average_hourly:   hourly,
            has_spiked:       false,
            spike_count:      0,
        }
    }

    /// Updates the listener data and determines if the feed has spiked
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

    /// Returns true if the specified feed is currently spiking in listeners
    /// based off of previous data collected by self.update().
    fn is_spiking(&self, feed: &Feed, config: &Config) -> bool {
        if self.average.current == 0.0 {
            return false;
        }

        let spike = config.get_feed_spike(&feed);
        let listeners = feed.listeners as f32;
        
        // If a feed has a low number of listeners, use a higher threshold to
        // make the calculation less sensitive to very small listener jumps
        let threshold = if listeners < 50.0 {
            spike.jump + (50.0 - listeners) * spike.low_listener_increase
        } else {
            // Otherwise, use a lower threshold based off of how fast the feed's
            // listeners are rising to encourage more updates during large incidents
            let delta = self.get_jump(feed.listeners);
            let rise_amount = delta / spike.high_listener_dec_every * spike.high_listener_dec;

            spike.jump - rise_amount.min(spike.jump - 0.01)
        };

        listeners - self.average.current >= listeners * threshold
    }

    fn update_listeners(&mut self, hour: usize, listeners: u32) {
        self.average.add_sample(listeners as i32);
        self.average_hourly[hour] = self.get_unskewed_avg();
    }

    fn update_unskewed_average(&mut self, listeners: u32, config: &Config) {
        if let Some(unskewed) = self.unskewed_average {
            // Remove the unskewed average when the current average is close
            if self.average.current - unskewed < unskewed * config.unskewed_avg.reset_pcnt {
                self.unskewed_average = None;
            } else {
                // Otherwise, slowly increase the unskewed average to adjust to
                // natural listener increases
                self.unskewed_average = Some(math::lerp(
                    unskewed,
                    self.average.current,
                    config.unskewed_avg.adjust_pcnt
                ));
            }
        } else if self.has_spiked && self.average.last > 0.0 {
            // This is used to set the unskewed average if the listener count is
            // much higher than the average to avoid polluting the average listener
            // count with a very high value
            let has_large_jump =
                listeners as f32 - self.average.current >
                self.average.last * config.unskewed_avg.jump_required;

            let has_spiked_enough = self.spike_count > config.unskewed_avg.spikes_required;

            if has_spiked_enough || has_large_jump {
                self.unskewed_average = Some(self.average.last);
            }
        }
    }

    /// Returns the unskewed average if it is set, or the current average otherwise.
    pub fn get_unskewed_avg(&self) -> f32 {
        self.unskewed_average.unwrap_or(self.average.current)
    }

    /// Returns the difference in listeners from the unskewed average.
    pub fn get_jump(&self, listeners: u32) -> f32 {
        listeners as f32 - self.get_unskewed_avg()
    }
}