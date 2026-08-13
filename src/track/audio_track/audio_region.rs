use crate::{
    data_types::Ticks,
    track::audio_track::{AudioDataInfo, AudioSource},
};
use serde::{Deserialize, Serialize};

/// Stores the raw audio source data.
#[derive(Clone, Serialize, Deserialize)]
pub struct AudioRegion {
    pub data_source: AudioSource,
    pub info: AudioDataInfo,
    pub start: Ticks,
    pub duration: Ticks,
    pub max_duration: Ticks,
}

impl AudioRegion {}
