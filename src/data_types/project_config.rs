use crate::data_types::{Beats, Ticks};
use serde::{Deserialize, Serialize};

#[derive(Clone, Default, Serialize, Deserialize, Debug)]
pub struct ProjectConfig {
    /// Represents how many ticks are in a single beat.
    pub resolution: u64,
    /// Number of channels of the project.
    pub channels: u16,
}

impl ProjectConfig {
    pub fn ticks_to_beats(&self, ticks: Ticks) -> Beats {
        Beats(ticks.0 as f64 / self.resolution as f64)
    }

    pub fn beats_to_ticks(&self, beats: Beats) -> Ticks {
        Ticks((beats.0 * self.resolution as f64) as i64)
    }
}
