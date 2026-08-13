use serde::{Deserialize, Serialize};
use std::{
    ops::Range,
    path::{Path, PathBuf},
};

#[derive(Clone, Serialize, Deserialize)]
pub struct AudioDataInfo {
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

impl AudioSource {
    /// Returns the full interleaved audio data for the given channel.
    pub fn get_data(&self) -> Option<Vec<f32>> {
        self.get_data_in(0..usize::MAX)
    }

    /// Returns the interleaved audio data for the given channel.
    pub fn get_data_in(&self, range: Range<usize>) -> Option<Vec<f32>> {
        match self {
            AudioSource::Original(path) => load_from_path(path, range),
            AudioSource::Modified(path) => load_from_path(path, range),
            AudioSource::Zero => None,
        }
    }
}

fn load_from_path(path: &Path, range: Range<usize>) -> Option<Vec<f32>> {
    let mut reader = hound::WavReader::open(path).ok()?;
    // Seek to the first sample in the range
    reader.seek(range.start as u32).ok()?;
    // Then read the samples in the range
    Some(
        reader
            .into_samples::<f32>()
            .take(range.count())
            .filter_map(Result::ok)
            .collect(),
    )
}
