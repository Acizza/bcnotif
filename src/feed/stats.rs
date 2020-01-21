use crate::config::Config;
use crate::database::listener_avgs;
use crate::database::Database;
use crate::err::Result;
use crate::feed::Feed;
use chrono::{NaiveDate, Utc};
use diesel::prelude::*;
use std::collections::HashMap;

/// Represents an average set of data that wraps around its specified sample size.
#[derive(Debug, Clone)]
pub struct Average {
    /// The current average. It is updated after calling to self.add_sample().
    pub current: f32,
    /// The current average before the last call to self.add_sample().
    pub last: f32,
    /// The raw data that is used to calculate the current and last average.
    pub data: [i32; Self::SAMPLE_SIZE],
    /// The current data index.
    index: usize,
    /// This keeps track of how many samples have been added since the struct
    /// was created, up until it reaches the specified sample size.
    /// It prevents data values that haven't been added from being averaged.
    populated: usize,
}

impl Average {
    pub const SAMPLE_SIZE: usize = 5;

    pub fn new() -> Self {
        Self::with_sample(0.0)
    }

    pub fn with_sample(value: f32) -> Self {
        Self {
            current: value,
            last: 0.0,
            data: [0; Self::SAMPLE_SIZE],
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
        Self::new()
    }
}

#[derive(Queryable, Insertable, Debug)]
pub struct ListenerAvg {
    pub id: i32,
    pub last_seen: NaiveDate,
    pub utc_0: Option<i32>,
    pub utc_4: Option<i32>,
    pub utc_8: Option<i32>,
    pub utc_12: Option<i32>,
    pub utc_16: Option<i32>,
    pub utc_20: Option<i32>,
}

impl ListenerAvg {
    pub fn new(id: i32) -> Self {
        Self {
            id,
            last_seen: Utc::now().naive_utc().date(),
            utc_0: None,
            utc_4: None,
            utc_8: None,
            utc_12: None,
            utc_16: None,
            utc_20: None,
        }
    }

    pub fn load_all(db: &Database) -> Result<ListenerAvgMap> {
        use crate::database::listener_avgs::dsl::*;

        let results = listener_avgs
            .load::<Self>(db.conn())?
            .into_iter()
            .map(|avg| (avg.id as u32, avg))
            .collect();

        Ok(results)
    }

    pub fn save_to_db(&self, db: &Database) -> diesel::QueryResult<usize> {
        use crate::database::listener_avgs::dsl::*;

        diesel::replace_into(listener_avgs)
            .values(self)
            .execute(db.conn())
    }

    pub fn for_hour(&self, hour: u32) -> Option<i32> {
        if hour < 4 || hour > 23 {
            self.utc_0
        } else if hour < 8 {
            self.utc_4
        } else if hour < 12 {
            self.utc_8
        } else if hour < 16 {
            self.utc_12
        } else if hour < 20 {
            self.utc_16
        } else {
            self.utc_20
        }
    }

    pub fn set_hour(&mut self, hour: u32, value: i32) {
        let avg = if hour < 4 || hour > 23 {
            &mut self.utc_0
        } else if hour < 8 {
            &mut self.utc_4
        } else if hour < 12 {
            &mut self.utc_8
        } else if hour < 16 {
            &mut self.utc_12
        } else if hour < 20 {
            &mut self.utc_16
        } else {
            &mut self.utc_20
        };

        *avg = Some(value);
        self.last_seen = Utc::now().naive_utc().date();
    }

    pub fn update_from_stats(&mut self, hour: u32, stats: &ListenerStats) {
        self.set_hour(hour, stats.get_unskewed_avg() as i32);
    }
}

pub type ListenerAvgMap = HashMap<u32, ListenerAvg>;

/// Represents general statistical data for feeds.
#[derive(Debug, Clone)]
pub struct ListenerStats {
    /// Represents the average number of listeners.
    pub average: Average,
    /// Represents the average number of listeners before a consistent spike occured.
    pub unskewed_average: Option<f32>,
    /// Indicates whether or not the listner count has spiked since the last update.
    pub has_spiked: bool,
    /// Represents the number of times the feed has spiked consecutively.
    pub spike_count: u32,
}

impl ListenerStats {
    pub fn new(listeners: u32) -> Self {
        Self {
            average: Average::with_sample(listeners as f32),
            unskewed_average: None,
            has_spiked: false,
            spike_count: 0,
        }
    }

    /// Updates the listener data and determines if the feed has spiked
    pub fn update(&mut self, feed: &Feed, config: &Config) {
        self.has_spiked = self.is_spiking(feed, config);

        self.spike_count = if self.has_spiked {
            self.spike_count + 1
        } else {
            0
        };

        self.average.add_sample(feed.listeners as i32);
        self.update_unskewed_average(feed.listeners as f32, config);
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

    pub fn should_display_feed(&self, feed: &Feed, config: &Config) -> bool {
        if let Some(max_times) = config.misc.max_times_to_show_feed {
            if self.spike_count > max_times {
                return false;
            }
        }

        let has_alert = feed.alert.is_some() && config.misc.show_alert_feeds;
        self.has_spiked || has_alert
    }
}

pub type ListenerStatMap = HashMap<u32, ListenerStats>;

fn lerp(v0: f32, v1: f32, t: f32) -> f32 {
    (1. - t) * v0 + t * v1
}
