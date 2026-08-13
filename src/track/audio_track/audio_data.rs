use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Serialize, Deserialize)]
pub(super) struct AudioDataInfo {
    /// The number of channels in the audio data.
    pub channels: usize,
    /// The number of samples in the audio data.
    pub frames: usize,
    /// The sample rate associated with the audio data.
    pub sample_rate: u64,
    /// The bpm associated with the audio data.
    pub bpm: f64,
}

#[derive(Clone, Serialize, Deserialize)]
pub enum AudioSource {
    /// The audio data is loaded from the original audio file.
    Original(PathBuf),
    /// The audio data is loaded from a modified copy of the file.
    Modified(PathBuf),
    /// The audio data is empty.
    Zero,
}
