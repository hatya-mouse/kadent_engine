use crate::{
    MAX_CHANNELS,
    audio_data::{AudioData, AudioFilePool, AudioSource},
    data_types::{PlaybackContext, Ticks},
    mixer::TempoMap,
    track::audio_track::resampler::resample_channels,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Stores the raw audio source data.
#[derive(Clone, Serialize, Deserialize)]
pub struct AudioRegion {
    pub data_source: AudioSource,
    pub start: Ticks,
    pub duration: Ticks,
    pub max_duration: Ticks,
    /// The bpm associated with the audio data. This is used to change the tempo when the tempo of the project is different.
    pub bpm: f64,

    /// Cached region start global sample index in the current playback context.
    #[serde(skip)]
    region_start_sample: usize,
    /// Cached region end global sample index in the current playback context.
    #[serde(skip)]
    region_end_sample: usize,
    /// Cached raw audio data for the region.
    #[serde(skip)]
    audio_data: Option<Arc<AudioData>>,
    /// Pre-allocated buffer for resampled audio data.
    #[serde(skip)]
    resampled_buffer: Vec<f32>,
}

impl AudioRegion {
    // --- INITIALIZER ---

    pub fn new(data_source: AudioSource, start: Ticks, duration: Ticks, bpm: f64) -> Self {
        let max_duration = duration;
        Self {
            data_source,
            start,
            duration,
            max_duration,
            bpm,
            region_start_sample: 0,
            region_end_sample: 0,
            audio_data: None,
            resampled_buffer: Vec::new(),
        }
    }

    pub fn zeros(start: Ticks, duration: Ticks, bpm: f64) -> Self {
        let max_duration = duration;
        let data_source = AudioSource::Zero;
        Self {
            data_source,
            start,
            duration,
            max_duration,
            bpm,
            region_start_sample: 0,
            region_end_sample: 0,
            audio_data: None,
            resampled_buffer: Vec::new(),
        }
    }

    // --- REGION PROCESSING ---

    pub(super) fn prepare(
        &mut self,
        audio_pool: &mut AudioFilePool,
        tempo_map: &TempoMap,
        playback_ctx: &PlaybackContext,
    ) {
        self.region_start_sample = tempo_map.ticks_to_samples(self.start);
        self.region_end_sample = tempo_map.ticks_to_samples(self.end());

        // Load the audio data from the audio pool
        self.audio_data = audio_pool.get_or_load(&self.data_source);

        if let Some(audio_data) = &self.audio_data {
            self.resampled_buffer
                .reserve(playback_ctx.buffer_size * audio_data.info.channels);
        } else {
            self.resampled_buffer.clear();
        }
    }

    /// Reads and writes the audio data to the given buffer based on the current playhead position and the buffer size.
    ///
    /// # Parameters
    /// - `playhead`: The current playhead position in the sample rate of current playback context.
    /// - `buffer`: The buffer to write the audio data into.
    /// - `tempo_map`: The tempo map for the current playback context.
    /// - `playback_ctx`: The playback context containing the sample rate and other playback settings.
    pub(super) fn render_buffer(
        &mut self,
        playhead: usize,
        buffer: &mut [f32],
        tempo_map: &TempoMap,
        playback_ctx: &PlaybackContext,
    ) {
        // Skip processing if the audio data is not loaded
        let Some(audio_data) = &self.audio_data else {
            return;
        };

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
        let local_start = tempo_map.global_to_local_sample(
            self.region_start_sample,
            global_start,
            &audio_data.info,
            self.bpm,
        );
        let sections =
            tempo_map.get_sections_in_range(global_start, global_end, &audio_data.info, self.bpm);
        let mut current_dst_offset = (global_start - playhead) * MAX_CHANNELS;

        for section in sections {
            if section.local_start_sample >= section.local_end_sample {
                continue;
            }

            // Get the audio data that corresponds to the current section
            // Add margin at the end for smoother lerp
            let data_start = (local_start + section.local_start_sample) * audio_data.info.channels;
            let data_end = (local_start + section.local_end_sample + 1) * audio_data.info.channels;
            if let Some(data) = audio_data.get_sample(data_start..data_end) {
                if data.is_empty() {
                    continue;
                };

                // Resample the audio data based on the resample ratio calculated by the tempo map
                if (section.resample_ratio - 1.0).abs() < 1e-6 {
                    self.resampled_buffer.clear();
                    self.resampled_buffer.extend_from_slice(data);
                } else {
                    resample_channels(
                        data,
                        &mut self.resampled_buffer,
                        playback_ctx.buffer_size,
                        audio_data.info.channels,
                        section.resample_ratio,
                    );
                };

                // Interleave and add the resampled data to the buffer, which must have MAX_CHANNELS channels
                if current_dst_offset < buffer.len() {
                    add_samples_interleaved(
                        &self.resampled_buffer,
                        &mut buffer[current_dst_offset..],
                        audio_data.info.channels,
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

/// Add the samples from the source buffer to the destination buffer while interleaving the channels
/// with the given number of source and destination channels.
pub(super) fn add_samples_interleaved(
    source: &[f32],
    destination: &mut [f32],
    src_channels: usize,
    dst_channels: usize,
) {
    let active_channels = src_channels.min(dst_channels);

    // Finally add the output data to the output buffer while interleaving the channels
    for (dst_frame, src_frame) in destination
        .chunks_exact_mut(dst_channels)
        .zip(source.chunks_exact(src_channels))
    {
        for ch in 0..active_channels {
            // Add the sample value
            dst_frame[ch] += src_frame[ch];
        }
    }
}
