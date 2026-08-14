mod audio_pool;
mod source;

use std::slice::SliceIndex;

pub(crate) use audio_pool::AudioFilePool;
pub use source::AudioSource;

#[derive(Debug, Clone)]
pub struct AudioDataInfo {
    /// The number of channels in the audio data.
    pub channels: usize,
    /// The number of samples in the audio data.
    pub frames: usize,
    /// The sample rate associated with the audio data.
    pub sample_rate: u64,
}

/// The raw audio data stored as a vector of f32 samples.
#[derive(Debug, Clone)]
pub struct AudioData {
    /// The raw audio samples.
    pub samples: Vec<f32>,
    /// The information about the audio data.
    pub info: AudioDataInfo,
}

impl AudioData {
    pub fn new(samples: Vec<f32>, info: AudioDataInfo) -> Self {
        Self { samples, info }
    }

    /// Get the slice of interleaves samples in the requested range.
    pub fn get_sample<I>(&self, index: I) -> Option<&I::Output>
    where
        I: SliceIndex<[f32]>,
    {
        self.samples.get(index)
    }
}
