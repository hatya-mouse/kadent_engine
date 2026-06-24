use crate::data_types::Ticks;
use serde::{Deserialize, Serialize};

/// Stores the raw audio source data.
#[derive(Clone, Serialize, Deserialize)]
pub struct AudioRegion {
    pub data: Vec<f32>,
    pub frames: usize,
    pub sample_rate: u32,
    pub channels: u16,
    pub base_bpm: f64,
    pub start: Ticks,
    pub duration: Ticks,
    pub max_duration: Ticks,
}

impl AudioRegion {
    /// Create a new audio region with zeros.
    pub fn zeros(
        frames: usize,
        sample_rate: u32,
        channels: u16,
        base_bpm: f64,
        start: Ticks,
        duration: Ticks,
    ) -> Self {
        Self {
            data: vec![0.0; frames * channels as usize],
            frames,
            sample_rate,
            channels,
            base_bpm,
            start,
            duration,
            max_duration: duration,
        }
    }
}
