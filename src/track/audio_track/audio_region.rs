use crate::{
    MAX_CHANNELS,
    data_types::{AudioContext, PlaybackContext, Ticks},
    mixer::TempoMap,
    track::audio_track::{
        AudioDataInfo, AudioSource,
        tempo_strech::{add_samples_interleaved, tempo_strech},
    },
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

    /// Cached factor used to convert relative global ticks to local sample indices in the audio data.
    ticks_to_local_factor: f64,
}

impl AudioRegion {
    // --- REGION PROCESSING ---

    pub(super) fn prepare(&mut self, audio_ctx: &AudioContext) {
        self.ticks_to_local_factor =
            (60f64 * self.info.sample_rate as f64) / (audio_ctx.resolution as f64 * self.info.bpm);
    }

    /// Reads and writes the audio data to the given buffer based on the current playhead position and the buffer size.
    ///
    /// # Parameters
    /// - `playhead`: The current playhead position in the sample rate of current playback context.
    /// - `buffer`: The buffer to write the audio data into.
    /// - `tempo_map`: The tempo map for the current playback context.
    /// - `playback_ctx`: The playback context containing the sample rate and other playback settings.
    pub(super) fn render_buffer(
        &self,
        playhead: usize,
        buffer: &mut [f32],
        tempo_map: &TempoMap,
        playback_ctx: &PlaybackContext,
    ) {
        // Convert the playhead position to a sample index in the audio data
        let local_start = self.calculate_local_samples(playhead, tempo_map);
        let local_end = self.calculate_local_samples(playhead + buffer.len(), tempo_map);

        // Get the slice of the audio data from the audio source
        let Some(data) = self.data_source.get_data(local_start..local_end) else {
            // If the data was None, do not write anything to the buffer and return
            return;
        };

        // Resample the audio data
        let resampled = tempo_strech(
            &data,
            &self.info,
            self.start,
            self.end(),
            playback_ctx.sample_rate,
            tempo_map,
        );

        // Interleave and add the resampled data to the buffer
        // The buffer must have MAX_CHANNELS channels
        add_samples_interleaved(&resampled, buffer, self.info.channels, MAX_CHANNELS);
    }

    // --- CALCULATIONS ---

    /// Returns the end position of the audio region in ticks.
    #[inline]
    fn end(&self) -> Ticks {
        self.start + self.duration
    }

    /// Converts the global sample index to the corresponding local sample index in the audio data.
    #[inline]
    fn calculate_local_samples(&self, global_samples: usize, tempo_map: &TempoMap) -> usize {
        let global_ticks = tempo_map.samples_to_ticks(global_samples);
        let delta_ticks = (global_ticks.0 - self.start.0) as f64;
        (delta_ticks * self.ticks_to_local_factor) as usize
    }
}
