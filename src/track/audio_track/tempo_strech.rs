use crate::{
    mixer::TempoMap,
    track::audio_track::{AudioDataInfo, resampler::resample_channels},
};

/// Strech the audio data using the given tempo map, not preserving the pitch.
/// The returned audio data will start at the beginning of the region.
///
/// # Parameters
/// - `src`: The source audio data to be streched.
/// - `src_info`: The information associated with the source audio data.
/// - `start_samples`: The start sample index of the region to be streched.
/// - `end_samples`: The end sample index of the region to be streched.
/// - `dst_sample_rate`: The sample rate of the destination audio data.
/// - `tempo_map`: The tempo map to be used for streching the audio data.
pub(super) fn tempo_strech(
    src: &[f32],
    src_info: &AudioDataInfo,
    start_samples: usize,
    end_samples: usize,
    dst_sample_rate: u64,
    tempo_map: &TempoMap,
) -> Vec<f32> {
    // If the destination sample rate is zero, return immediately with an empty vector
    if src.is_empty() || dst_sample_rate == 0 {
        return Vec::new();
    }

    // Create a section list by splitting the region into sections based on tempo change events
    // Get the first event on or before the region start beat
    let start_index = tempo_map
        .events
        .partition_point(|e| e.sample_offset() <= start_samples)
        .saturating_sub(1);
    // Loop over the events until it surpasses the region end beat
    // and create sections that has to be resampled separately
    // (0: Start sample index, 1: End sample index, 2: BPM of the section)
    let mut sections: Vec<(usize, usize, f64)> = Vec::with_capacity(16);
    let mut i = start_index;

    // Loop over the tempo change events and create sections based on the tempo changes
    while let Some(event) = tempo_map.events.get(i) {
        // Break if the event beat surpasses the region end beat
        if event.sample_offset() >= end_samples {
            break;
        }

        // Get the start and the end beat of the section
        let section_start = if i == start_index {
            start_samples
        } else {
            event.sample_offset()
        };
        let section_end = tempo_map
            .events
            .get(i + 1)
            .map(|next| next.sample_offset().min(end_samples))
            .unwrap_or(end_samples);

        // Push the section
        sections.push((section_start, section_end, event.bpm()));
        i += 1;
    }

    // Resample each tempo section and append it to the output data
    let mut output_data = Vec::new();
    for section in sections {
        let resample_ratio =
            src_info.sample_rate as f64 * src_info.bpm / (section.2 * dst_sample_rate as f64);
        let inv_resample_ratio = 1.0 / resample_ratio;

        // Calculate the relative start and the end index by subtracting by the start index of the part to be resampled
        let src_start_sample = (section.0.saturating_sub(start_samples) as f64 * inv_resample_ratio)
            .min(src_info.frames as f64) as usize;
        let src_end_sample = (section.1.saturating_sub(start_samples) as f64 * inv_resample_ratio)
            .min(src_info.frames as f64) as usize;

        // Calculate the number of samples in the section and skip if it's zero
        let section_samples = src_end_sample - src_start_sample;
        if section_samples == 0 {
            continue;
        }

        // Calculate the start and end index of the section in the source buffer
        let src_start_index = src_start_sample * src_info.channels;
        let src_end_index = src_end_sample * src_info.channels;

        // Then get the section data from the source buffer
        let section_data = &src.get(src_start_index..src_end_index).unwrap_or(&[]);
        println!(
            "src_start_index: {}, src_end_index: {}",
            src_start_index, src_end_index
        );

        // Calculate the ratio of the sample rate and the bpm
        let resampled_data = resample_channels(section_data, src_info.channels, resample_ratio);

        // Append the resampled audio to the output data
        output_data.extend(resampled_data);
    }

    output_data
}

/// Add the samples from the source buffer to the destination buffer while interleaving the channels
/// with the given number of source and destination channels.
pub(super) fn add_samples_interleaved(
    source: &[f32],
    destination: &mut [f32],
    src_channels: usize,
    dst_channels: usize,
) {
    let active_channels = src_channels.min(src_channels);

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
