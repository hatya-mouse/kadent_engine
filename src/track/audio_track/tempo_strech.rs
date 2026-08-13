use crate::{
    data_types::Ticks,
    mixer::TempoMap,
    track::audio_track::{AudioDataInfo, resampler::resample_channels},
};

/// Strech the audio data using the given tempo map, not preserving the pitch.
/// The returned audio data will start at the beginning of the region.
///
/// # Arguments
/// - `src`: The source audio data to be streched.
/// - `src_info`: The information associated with the source audio data.
/// - `start_ticks`: The start tick of the region to be streched.
/// - `end_ticks`: The end tick of the region to be streched.
/// - `dst_sample_rate`: The sample rate of the destination audio data.
/// - `tempo_map`: The tempo map to be used for streching the audio data.
pub(super) fn tempo_strech(
    src: &[f32],
    src_info: AudioDataInfo,
    start_ticks: Ticks,
    end_ticks: Ticks,
    dst_sample_rate: u64,
    tempo_map: &TempoMap,
) -> Vec<f32> {
    // If the destination sample rate is zero, return immediately with an empty vector
    if dst_sample_rate == 0 {
        return Vec::new();
    }

    // Create a section list by splitting the region into sections based on tempo change events
    // Get the first event on or before the region start beat
    let start_index = tempo_map
        .events
        .partition_point(|e| e.ticks() <= start_ticks)
        .saturating_sub(1);
    // Loop over the events until it surpasses the region end beat
    // (0: Start ticks, 1: End ticks, 2: BPM of the section)
    // Allocate a vector with a capacity of 16 for now,
    // assuming the number of tempo change in a region is not so large in common cases
    let mut sections: Vec<(Ticks, Ticks, f64)> = Vec::with_capacity(16);
    let mut i = start_index;

    // Loop over the tempo change events and create sections based on the tempo changes
    while let Some(event) = tempo_map.events.get(i) {
        // Break if the event beat surpasses the region end beat
        if event.ticks() >= end_ticks {
            break;
        }

        // Get the start and the end beat of the section
        let section_start = if i == start_index {
            start_ticks
        } else {
            event.ticks()
        };
        let section_end = tempo_map
            .events
            .get(i + 1)
            .map(|next| next.ticks().min(end_ticks))
            .unwrap_or(end_ticks);

        // Push the section
        sections.push((section_start, section_end, event.bpm()));
        i += 1;
    }

    // Loop over the sections and resample the audio
    // `src_region.data` is indexed relative to the region's own start, not to the absolute project time,
    // so section ticks must be offset by the region's start sample
    let region_start_sample = tempo_map.ticks_to_samples(start_ticks);

    let mut output_data = Vec::new();

    // Resample each tempo section and append it to the output data
    for section in sections {
        // Calculate the relative start and the end index
        let src_start_sample = tempo_map
            .ticks_to_samples(section.0)
            .saturating_sub(region_start_sample)
            .min(src_info.frames);
        let src_end_sample = tempo_map
            .ticks_to_samples(section.1)
            .saturating_sub(region_start_sample)
            .min(src_info.frames);

        // Calculate the number of samples in the section and skip if it's zero
        let section_samples = src_end_sample - src_start_sample;
        if section_samples == 0 {
            continue;
        }

        // Calculate the start and end index of the section in the source buffer
        let src_start_index = src_start_sample * src_info.channels;
        let src_end_index = src_end_sample * src_info.channels;

        // Then get the section data from the source buffer
        let section_data = &src[src_start_index..src_end_index];

        // Calculate the source sample rate to change the tempo
        let resample_ratio =
            src_info.sample_rate as f64 * src_info.bpm / section.2 * dst_sample_rate as f64;
        let resampled_data = resample_channels(
            section_data,
            src_info.channels,
            section_samples,
            resample_ratio,
        );

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
