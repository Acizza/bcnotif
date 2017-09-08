extern crate csv;
extern crate chrono;

use std::collections::{HashMap, VecDeque};
use std::path::Path;
use config::Config;
use feed::Feed;
use util::lerp;
use self::chrono::prelude::{Utc, Timelike};

const MOVING_AVG_SIZE: usize = 5;

error_chain! {}

#[derive(Debug)]
pub struct Average {
    pub current: f32,
    pub last:    f32,
    moving:      VecDeque<f32>,
}

impl Average {
    pub fn new(average: f32) -> Average {
        let mut moving = VecDeque::with_capacity(MOVING_AVG_SIZE + 1);
        
        if average > 0. {
            moving.push_back(average);
        }

        Average {
            current: average,
            last:    average,
            moving:  moving,
        }
    }

    fn update(&mut self, value: f32) {
        self.moving.push_back(value);

        if self.moving.len() > MOVING_AVG_SIZE {
            self.moving.pop_front();
        }

        self.last    = self.current;
        self.current = self.moving.iter().sum::<f32>() / self.moving.len() as f32;
    }
}

#[derive(Debug)]
pub struct ListenerData {
    pub average:      Average,
    pub unskewed_avg: Option<f32>,
    pub hourly:       [f32; 24],
    spike_count:      u8,
}

impl ListenerData {
    pub fn new(listeners: f32, hourly: [f32; 24]) -> ListenerData {
        ListenerData {
            average:      Average::new(listeners),
            unskewed_avg: None,
            hourly:       hourly,
            spike_count:  0,
        }
    }

    pub fn step(&mut self, config: &Config, hour: usize, feed: &Feed) -> bool {
        let has_spiked = self.has_spiked(&config, &feed);

        self.spike_count = if has_spiked {
            self.spike_count + 1
        } else {
            0
        };

        let listeners = feed.listeners as f32;
        self.average.update(listeners);
        self.update_unskewed(&config, listeners, has_spiked);

        self.hourly[hour] = match self.unskewed_avg {
            Some(unskewed) => unskewed,
            None           => self.average.current,
        };

        has_spiked
    }

    fn update_unskewed(&mut self, config: &Config, listeners: f32, has_spiked: bool) {
        match self.unskewed_avg {
            Some(unskewed) => {
                // Remove the unskewed average when the current average is close
                if self.average.current - unskewed < unskewed * config.unskewed_avg.reset_pcnt {
                    self.unskewed_avg = None;
                } else {
                    // Slowly adjust the unskewed average to adjust to any natural listener increases
                    let new_val = lerp(unskewed,
                                        self.average.current,
                                        config.unskewed_avg.adjust_pcnt);

                    self.unskewed_avg = Some(new_val);
                }
            },
            None if has_spiked && self.average.last > 0. => {
                let has_spiked_enough = self.spike_count > config.unskewed_avg.spikes_required;

                // The main purpose of this is to "capture" feeds on the first update cycle
                // that are hundreds if not thousands of listeners above normal
                let has_jumped_enough =
                    listeners - self.average.current >
                    self.average.last * config.unskewed_avg.jump_required;

                if has_spiked_enough || has_jumped_enough {
                    self.unskewed_avg = Some(self.average.last);
                }
            },
            None => (),
        }
    }

    pub fn has_spiked(&self, config: &Config, feed: &Feed) -> bool {
        if self.average.current == 0. {
            return false
        }

        let spike = config.get_current_spike(&feed);
        let listeners = feed.listeners as f32;

        // If a feed has a low number of listeners, make the threshold higher to
        // make the calculation less sensitive to very small listener jumps
        let threshold = if listeners < 50. {
            spike.jump + (50. - listeners) * spike.low_listener_increase
        } else {
            // Otherwise, decrease the threshold by a factor of how fast the feed's listeners are rising
            // to make it easier for the feed to show up in an update
            let pcnt     = spike.high_listener_dec;
            let per_pcnt = spike.high_listener_dec_every;

            spike.jump - (self.get_average_delta(listeners) / per_pcnt * pcnt).min(spike.jump - 0.01)
        };
        
        if cfg!(feature = "show-feed-info") {
            print!(" THR: {}", threshold);
        }
        
        listeners - self.average.current >= listeners * threshold
    }

    pub fn get_average_delta(&self, listeners: f32) -> f32 {
        let sub = match self.unskewed_avg {
            Some(unskewed) => unskewed,
            None => self.average.current,
        };

        listeners - sub
    }
}

pub type AverageMap = HashMap<u32, ListenerData>;

pub fn load_averages(path: &Path) -> Result<AverageMap> {
    let reader = csv::ReaderBuilder::new()
        .has_headers(false)
        .from_path(path)
        .chain_err(|| "failed to open listener average file")?;

    let mut avgs = HashMap::new();
    let hour = Utc::now().hour() as usize;

    for record in reader.into_deserialize() {
        let (id, avg): (_, [_; 24]) = record
            .chain_err(|| "failed to decode listener average")?;

        avgs.insert(id, ListenerData::new(avg[hour], avg));
    }

    Ok(avgs)
}

pub fn save_averages(path: &Path, averages: &AverageMap) -> Result<()> {
    let mut writer = csv::Writer::from_path(path)
        .chain_err(|| "failed to open listener average file")?;
    
    for (id, data) in averages {
        let mut record = Vec::with_capacity(data.hourly.len() + 1);

        record.push(id.to_string());

        for hour_data in data.hourly.iter() {
            record.push(hour_data.to_string());
        }

        writer.write_record(&record)
            .chain_err(|| "failed to encode listener average")?;
    }

    Ok(())
}