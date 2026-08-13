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
    /// Cached region start global sample index in the current playback context.
    region_start_samples: usize,
    /// Cached region end global sample index in the current playback context.
    region_end_samples: usize,
}

impl AudioRegion {
    // --- INITIALIZER ---

    pub fn new(
        data_source: AudioSource,
        info: AudioDataInfo,
        start: Ticks,
        duration: Ticks,
    ) -> Self {
        let max_duration = duration;
        Self {
            data_source,
            info,
            start,
            duration,
            max_duration,
            ticks_to_local_factor: 0.0,
            region_start_samples: 0,
            region_end_samples: 0,
        }
    }

    pub fn zeros(info: AudioDataInfo, start: Ticks, duration: Ticks) -> Self {
        let max_duration = duration;
        let data_source = AudioSource::Zero;
        Self {
            data_source,
            info,
            start,
            duration,
            max_duration,
            ticks_to_local_factor: 0.0,
            region_start_samples: 0,
            region_end_samples: 0,
        }
    }

    // --- REGION PROCESSING ---

    pub(super) fn prepare(&mut self, tempo_map: &TempoMap, audio_ctx: &AudioContext) {
        self.ticks_to_local_factor =
            (60.0 * self.info.sample_rate as f64) / (audio_ctx.resolution as f64 * self.info.bpm);
        self.region_start_samples = tempo_map.ticks_to_samples(self.start);
        self.region_end_samples = tempo_map.ticks_to_samples(self.end());
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
        // Skip processing if the buffer falls entirely outside the region's range
        let buffer_end = playhead + playback_ctx.buffer_size;
        if buffer_end <= self.region_start_samples || playhead >= self.region_end_samples {
            return;
        }

        // Calculate where to start and end reading from the audio data
        // This is to handle edge cases
        // Buffer: |<-- Current buffer block -->|
        //         ^ playhead                   ^ buffer_end
        // Region:      [<-- Region -->]
        // global_start ^              ^ global_end
        let global_start = playhead.max(self.region_start_samples);
        let global_end = buffer_end.min(self.region_end_samples);

        // Convert the playhead position to a sample index in the audio data
        let data_start = self.calculate_local_samples(global_start, tempo_map) * self.info.channels;
        let data_end = self.calculate_local_samples(global_end, tempo_map) * self.info.channels;
        if data_start >= data_end {
            return;
        }

        // Get the slice of the audio data from the audio source
        let Some(data) = self.data_source.get_data_in(data_start..data_end) else {
            // If the data was None, do not write anything to the buffer and return
            return;
        };
        println!("data: {}", data.len());

        // Resample the audio data
        let resampled = tempo_strech(
            &data,
            &self.info,
            self.start,
            self.end(),
            playback_ctx.sample_rate,
            tempo_map,
        );

        // Calculate the output buffer offset where writing should start
        let buffer_offset = self.region_start_samples.saturating_sub(playhead) * MAX_CHANNELS;
        // Interleave and add the resampled data to the buffer; The buffer must have MAX_CHANNELS channels
        if buffer_offset < buffer.len() {
            add_samples_interleaved(
                &resampled,
                &mut buffer[buffer_offset..],
                self.info.channels,
                MAX_CHANNELS,
            );
        }
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
        let delta_ticks = global_ticks.0.saturating_sub(self.start.0) as f64;
        (delta_ticks * self.ticks_to_local_factor) as usize
    }
}
