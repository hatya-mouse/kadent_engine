use crate::data_types::{Beats, Ticks};
use serde::{Deserialize, Serialize};

#[derive(Clone, Default, Serialize, Deserialize, Debug)]
pub struct AudioContext {
    /// Represents how many ticks are in one beat.
    pub resolution: u64,
    pub channels: usize,
    pub sample_rate: u64,
    pub buffer_size: usize,
    pub max_voices: usize,
}

impl AudioContext {
    pub fn ticks_to_beats(&self, ticks: Ticks) -> Beats {
        Beats(ticks.0 as f64 / self.resolution as f64)
    }

    pub fn beats_to_ticks(&self, beats: Beats) -> Ticks {
        Ticks((beats.0 * self.resolution as f64) as i64)
    }
}
