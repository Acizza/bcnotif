use crate::config::Config;
use crate::err::{self, Result};
use crate::feed::Feed;
use crate::path::FilePath;
use smallvec::{smallvec, SmallVec};
use snafu::OptionExt;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;

/// Represents an average set of data that wraps around its specified sample size.
#[derive(Debug, Clone)]
pub struct Average {
    /// The current average. It is updated after calling to self.add_sample().
    pub current: f32,
    /// The current average before the last call to self.add_sample().
    pub last: f32,
    /// The raw data that is used to calculate the current and last average.
    pub data: SmallVec<[i32; 5]>,
    /// The current data index.
    index: usize,
    /// This keeps track of how many samples have been added since the struct
    /// was created, up until it reaches the specified sample size.
    /// It prevents data values that haven't been added from being averaged.
    populated: usize,
}

impl Average {
    pub const DEFAULT_SAMPLE_SIZE: usize = 5;

    pub fn new(sample_size: usize) -> Average {
        Average {
            current: 0.0,
            last: 0.0,
            data: smallvec![0; sample_size],
            index: 0,
            populated: 0,
        }
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

impl Default for Average {
    fn default() -> Self {
        Average::new(Average::DEFAULT_SAMPLE_SIZE)
    }
}

pub type HourlyListeners = [f32; 24];

/// Represents general statistical data for feeds.
#[derive(Debug, Clone)]
pub struct ListenerStats {
    /// Represents the average number of listeners.
    pub average: Average,
    /// Represents the average number of listeners before a consistent spike occured.
    pub unskewed_average: Option<f32>,
    /// Contains the average number of listeners for any given hour.
    pub average_hourly: HourlyListeners,
    /// Indicates whether or not the listner count has spiked since the last update.
    pub has_spiked: bool,
    /// Represents the number of times the feed has spiked consecutively.
    pub spike_count: u32,
}

impl ListenerStats {
    pub fn new() -> Self {
        Self::with_hourly([0.0; 24])
    }

    /// Create a new `ListenerStats` struct with existing hourly data.
    pub fn with_hourly(hourly: HourlyListeners) -> Self {
        Self {
            average: Average::default(),
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

        let spike = config.get_feed_spike(feed);
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
                self.unskewed_average = Some(lerp(
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

fn lerp(v0: f32, v1: f32, t: f32) -> f32 {
    (1. - t) * v0 + t * v1
}

#[derive(Debug)]
pub struct ListenerStatMap(HashMap<u32, ListenerStats>);

impl ListenerStatMap {
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    pub fn validated_path() -> Result<PathBuf> {
        let mut path = FilePath::LocalData.validated_dir_path()?;
        path.push("averages.csv");
        Ok(path)
    }

    pub fn load_or_new() -> Result<Self> {
        match Self::load() {
            Ok(stats) => Ok(stats),
            Err(err) if err.is_file_nonexistant() => Ok(Self::new()),
            err => err,
        }
    }

    pub fn load() -> Result<Self> {
        let path = Self::validated_path()?;
        let reader = BufReader::new(File::open(&path)?);
        let mut stats = HashMap::with_capacity(1000);

        for line in reader.lines() {
            let line = line?;
            let mut columns = line.split(',');

            let id = columns
                .next()
                .and_then(|column| column.parse().ok())
                .context(err::MalformedCSV)?;

            let average_hourly = {
                let mut arr: HourlyListeners = [0.0; 24];

                for avg in &mut arr {
                    *avg = columns
                        .next()
                        .and_then(|column| column.parse().ok())
                        .context(err::MalformedCSV)?;
                }

                arr
            };

            let stat = ListenerStats::with_hourly(average_hourly);
            stats.insert(id, stat);
        }

        Ok(Self(stats))
    }

    pub fn save(&self) -> Result<()> {
        let mut buffer = String::new();

        for (id, stats) in self.stats() {
            buffer.push_str(&format!("{}", id));

            for &avg in &stats.average_hourly {
                buffer.push_str(&format!(",{}", avg as i32));
            }

            buffer.push('\n');
        }

        let path = Self::validated_path()?;
        std::fs::write(path, buffer).map_err(Into::into)
    }

    #[inline(always)]
    pub fn stats(&self) -> &HashMap<u32, ListenerStats> {
        &self.0
    }

    #[inline(always)]
    pub fn stats_mut(&mut self) -> &mut HashMap<u32, ListenerStats> {
        &mut self.0
    }
}
