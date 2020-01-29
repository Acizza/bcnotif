use crate::config::Config;
use crate::database::listener_avgs;
use crate::database::Database;
use crate::err::Result;
use crate::feed::Feed;
use chrono::{Duration, Utc, Weekday};
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
    pub last_seen: i64,
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
            last_seen: Utc::now().timestamp(),
            utc_0: None,
            utc_4: None,
            utc_8: None,
            utc_12: None,
            utc_16: None,
            utc_20: None,
        }
    }

    pub fn load(db: &Database, feed_id: i32) -> Result<Self> {
        use crate::database::listener_avgs::dsl::*;

        listener_avgs
            .filter(id.eq(feed_id))
            .get_result(db.conn())
            .map_err(Into::into)
    }

    pub fn load_or_new(db: &Database, feed_id: i32) -> Self {
        Self::load(db, feed_id).unwrap_or_else(|_| Self::new(feed_id))
    }

    pub fn save_to_db(&self, db: &Database) -> diesel::QueryResult<usize> {
        use crate::database::listener_avgs::dsl::*;

        diesel::replace_into(listener_avgs)
            .values(self)
            .execute(db.conn())
    }

    pub fn remove_old_from_db(db: &Database) -> diesel::QueryResult<usize> {
        use crate::database::listener_avgs::dsl::*;

        let today = Utc::now();
        let oldest_date = (today - Duration::days(30)).timestamp();

        diesel::delete(listener_avgs.filter(last_seen.lt(oldest_date))).execute(db.conn())
    }

    pub fn for_hour(&self, hour: u8) -> Option<i32> {
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

    pub fn set_hour(&mut self, hour: u8, value: i32) {
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
        self.last_seen = Utc::now().timestamp();
    }
}

/// Represents general statistical data for feeds.
#[derive(Debug)]
pub struct ListenerStats {
    /// The historical listener averages.
    pub listener_avg: ListenerAvg,
    /// Represents the average number of listeners.
    pub average: Average,
    /// Represents the average number of listeners before a consistent spike occured.
    pub unskewed_average: Option<f32>,
    /// The number of listeners the feed has jumped by since the last update.
    pub jump: f32,
    /// Indicates whether or not the listner count has spiked since the last update.
    pub has_spiked: bool,
    /// Represents the number of times the feed has spiked consecutively.
    pub spike_count: u32,
}

impl ListenerStats {
    const LOW_LISTENER_INCREASE: f32 = 0.005;
    const HIGH_LISTENER_DEC: f32 = 0.02;
    const HIGH_LISTENER_DEC_PER_LISTENERS: f32 = 100.0;

    const RESET_UNSKEWED_AVG_PCNT: f32 = 0.15;
    const JUMP_TO_SET_UNSKEWED_AVG: f32 = 4.0;
    const UNSKEWED_ADJUST_PCNT: f32 = 0.0075;
    const UNSKEWED_SPIKES_REQUIRED: u32 = 1;

    pub fn init_from_db(db: &Database, hour: u8, feed_id: i32, cur_listeners: f32) -> Self {
        let listener_avg = ListenerAvg::load_or_new(db, feed_id);

        let listeners = listener_avg
            .for_hour(hour)
            .map(|l| l as f32)
            .unwrap_or(cur_listeners);

        Self {
            listener_avg,
            average: Average::with_sample(listeners as f32),
            unskewed_average: None,
            jump: 0.0,
            has_spiked: false,
            spike_count: 0,
        }
    }

    /// Updates the listener data and determines if the feed has spiked
    pub fn update(&mut self, hour: u8, feed: &Feed, config: &Config, weekday: Weekday) {
        self.jump = feed.listeners as f32 - self.current_listener_average();
        self.has_spiked = self.is_spiking(feed, config, weekday);

        self.spike_count = if self.has_spiked {
            self.spike_count + 1
        } else {
            0
        };

        self.average.add_sample(feed.listeners as i32);
        self.update_unskewed_average(feed.listeners as f32);

        self.listener_avg
            .set_hour(hour, self.current_listener_average() as i32);
    }

    /// Returns true if the specified feed is currently spiking in listeners
    /// based off of previous data collected by self.update().
    fn is_spiking(&self, feed: &Feed, config: &Config, weekday: Weekday) -> bool {
        if self.average.current == 0.0 {
            return false;
        }

        let feed_cfg = config.options_for_feed(feed, weekday);
        let jump_required = feed_cfg.jump_required.as_mult();
        let listeners = feed.listeners as f32;

        // If a feed has a low number of listeners, use a higher threshold to
        // make the calculation less sensitive to very small listener jumps
        let threshold = if listeners < 50.0 {
            jump_required + (50.0 - listeners) * Self::LOW_LISTENER_INCREASE
        } else {
            // Otherwise, use a lower threshold based off of how fast the feed's
            // listeners are rising to encourage more updates during large incidents
            let rise_amount =
                self.jump / Self::HIGH_LISTENER_DEC_PER_LISTENERS * Self::HIGH_LISTENER_DEC;

            jump_required - rise_amount.min(jump_required - 0.01)
        };

        listeners - self.average.current >= listeners * threshold
    }

    fn update_unskewed_average(&mut self, listeners: f32) {
        if let Some(unskewed) = self.unskewed_average {
            // Remove the unskewed average if the current average is close to it
            if self.average.current - unskewed < unskewed * Self::RESET_UNSKEWED_AVG_PCNT {
                self.unskewed_average = None;
                return;
            }

            // Otherwise, if there isn't a huge jump in listeners, slowly increase
            // the unskewed average to adjust to natural listener increases
            if listeners - unskewed < unskewed * Self::JUMP_TO_SET_UNSKEWED_AVG {
                self.unskewed_average = Some(lerp(
                    unskewed,
                    self.average.current,
                    Self::UNSKEWED_ADJUST_PCNT,
                ));
            }
        } else if self.has_spiked && self.average.last > 0.0 {
            // This is used to set the unskewed average if the listener count is
            // much higher than the average to avoid polluting the average listener
            // count with a very high value
            let has_large_jump = listeners > self.average.last * Self::JUMP_TO_SET_UNSKEWED_AVG;
            let has_spiked_enough = self.spike_count > Self::UNSKEWED_SPIKES_REQUIRED;

            if has_spiked_enough || has_large_jump {
                self.unskewed_average = Some(self.average.last);
            }
        }
    }

    /// Returns a listener average that is resiliant to large sudden jumps.
    ///
    /// This is useful for preserving the integrity of the listener average over time.
    pub fn current_listener_average(&self) -> f32 {
        self.unskewed_average.unwrap_or(self.average.current)
    }

    pub fn should_display_feed(&self, feed: &Feed, config: &Config) -> bool {
        if let Some(max_times) = config.misc.show_max_times {
            if self.spike_count > max_times {
                return false;
            }
        }

        let has_alert = feed.alert.is_some() && config.misc.show_alert_feeds;
        self.has_spiked || has_alert
    }

    pub fn save_to_db(&self, db: &Database) -> diesel::QueryResult<usize> {
        self.listener_avg.save_to_db(db)
    }
}

pub type ListenerStatMap = HashMap<u32, ListenerStats>;

fn lerp(v0: f32, v1: f32, t: f32) -> f32 {
    (1. - t) * v0 + t * v1
}
