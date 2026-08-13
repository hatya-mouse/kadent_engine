use crate::{
    MAX_CHANNELS,
    data_types::{PlaybackContext, Ticks},
    mixer::TempoMap,
    track::audio_track::{
        AudioDataInfo, AudioSource, resampler::resample_channels,
        tempo_strech::add_samples_interleaved,
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

    /// Cached region start global sample index in the current playback context.
    #[serde(skip)]
    region_start_sample: usize,
    /// Cached region end global sample index in the current playback context.
    #[serde(skip)]
    region_end_sample: usize,
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
            region_start_sample: 0,
            region_end_sample: 0,
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
            region_start_sample: 0,
            region_end_sample: 0,
        }
    }

    // --- REGION PROCESSING ---

    pub(super) fn prepare(&mut self, tempo_map: &TempoMap) {
        self.region_start_sample = tempo_map.ticks_to_samples(self.start);
        self.region_end_sample = tempo_map.ticks_to_samples(self.end());
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
        if buffer_end <= self.region_start_sample || playhead >= self.region_end_sample {
            return;
        }

        // Calculate where to start and end reading from the audio data
        // This is to handle edge cases
        // Buffer: |<-- Current buffer block -->|
        //         ^ playhead                   ^ buffer_end
        // Region:      [<-- Region -->]
        // global_start ^              ^ global_end
        let global_start = playhead.max(self.region_start_sample);
        let global_end = buffer_end.min(self.region_end_sample);

        // Get the tempo sections from the tempo map
        let sections = tempo_map.get_sections_in_range(global_start, global_end, &self.info);
        let mut current_dst_offset = (global_start - playhead) * MAX_CHANNELS;

        for section in sections {
            if section.local_start_sample >= section.local_end_sample {
                continue;
            }

            // Get the audio data that corresponds to the current section
            let data_start = section.local_start_sample * self.info.channels;
            let data_end = section.local_end_sample * self.info.channels;
            if let Some(data) = self.data_source.get_data_in(data_start..data_end) {
                // Resample the audio data based on the resample ratio calculated by the tempo map
                let resampled = if (section.resample_ratio - 1.0).abs() < 1e-6 {
                    data.to_vec()
                } else {
                    resample_channels(&data, self.info.channels, section.resample_ratio)
                };

                // Interleave and add the resampled data to the buffer, which must have MAX_CHANNELS channels
                if current_dst_offset < buffer.len() {
                    add_samples_interleaved(
                        &resampled,
                        &mut buffer[current_dst_offset..],
                        self.info.channels,
                        MAX_CHANNELS,
                    );
                }

                // Advance the destination offset
                let rendered_frames = section.global_end_sample - section.global_start_sample;
                current_dst_offset += rendered_frames * MAX_CHANNELS;
            }
        }
    }

    // --- CALCULATIONS ---

    /// Returns the end position of the audio region in ticks.
    #[inline]
    fn end(&self) -> Ticks {
        self.start + self.duration
    }
}
