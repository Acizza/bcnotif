extern crate csv;

use config::Config;
use chrono::{Timelike, Utc};
use failure::Error;
use feed::Feed;
use std::collections::HashMap;
use std::path::PathBuf;
use math;

#[derive(Fail, Debug)]
pub enum AverageDataError {
    #[fail(display = "csv file contains record with too few rows")] TooFewRows,
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

    pub fn load(&mut self) -> Result<(), Error> {
        let hour = Utc::now().hour() as usize;
        let mut rdr = csv::Reader::from_path(&self.path)?;

        for result in rdr.records() {
            let record = result?;

            if record.len() < 1 + ListenerStats::HOURLY_SIZE {
                bail!(AverageDataError::TooFewRows);
            }

            let id = record[0].parse()?;
            let mut averages = [0.0; ListenerStats::HOURLY_SIZE];

            for i in 0..ListenerStats::HOURLY_SIZE {
                // Use an offset of 1 to avoid capturing the feed ID field
                averages[i] = record[1 + i].parse::<f32>()?;
            }

            let listeners = averages[hour] as i32;

            // Zero listeners means that data for the current hour doesn't exist yet
            let stats = if listeners == 0 {
                ListenerStats::with_hourly(averages)
            } else {
                ListenerStats::with_data(listeners, averages)
            };

            self.data.insert(id, stats);
        }

        Ok(())
    }

    pub fn save(&self) -> Result<(), csv::Error> {
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
            current: 0.0,
            last: 0.0,
            data: vec![0; sample_size],
            index: 0,
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

        self.current =
            self.data.iter().take(self.populated).sum::<i32>() as f32 / self.populated as f32;
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
    const HOURLY_SIZE: usize = 24;

    pub fn new() -> ListenerStats {
        ListenerStats::with_hourly([0.0; ListenerStats::HOURLY_SIZE])
    }

    /// Creates a new ListenerStats struct with existing hourly data.
    pub fn with_hourly(hourly: [f32; ListenerStats::HOURLY_SIZE]) -> ListenerStats {
        ListenerStats {
            average: Average::new(ListenerStats::AVERAGE_SIZE),
            unskewed_average: None,
            average_hourly: hourly,
            has_spiked: false,
            spike_count: 0,
        }
    }

    /// Creates a new ListenerStats struct with existing listener and hourly listener data.
    pub fn with_data(listeners: i32, hourly: [f32; ListenerStats::HOURLY_SIZE]) -> ListenerStats {
        ListenerStats {
            average: Average::with_value(ListenerStats::AVERAGE_SIZE, listeners),
            unskewed_average: None,
            average_hourly: hourly,
            has_spiked: false,
            spike_count: 0,
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

        self.average.add_sample(feed.listeners as i32);
        self.update_unskewed_average(feed.listeners as f32, config);
        self.average_hourly[hour] = self.get_unskewed_avg();
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

    fn update_unskewed_average(&mut self, listeners: f32, config: &Config) {
        if let Some(unskewed) = self.unskewed_average {
            // Remove the unskewed average if the current average is close to it
            if self.average.current - unskewed < unskewed * config.unskewed_avg.reset_pcnt {
                self.unskewed_average = None;
                return;
            }

            // Otherwise, if there isn't a huge jump in listeners, slowly increase
            // the unskewed average to adjust to natural listener increases
            if listeners - unskewed < unskewed * config.unskewed_avg.jump_required {
                self.unskewed_average = Some(math::lerp(
                    unskewed,
                    self.average.current,
                    config.unskewed_avg.adjust_pcnt,
                ));
            }
        } else if self.has_spiked && self.average.last > 0.0 {
            // This is used to set the unskewed average if the listener count is
            // much higher than the average to avoid polluting the average listener
            // count with a very high value
            let has_large_jump = listeners > self.average.last * config.unskewed_avg.jump_required;

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
