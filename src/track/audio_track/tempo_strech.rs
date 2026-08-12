use crate::{
    data_types::Ticks,
    mixer::TempoMap,
    track::audio_track::{AudioRegion, resampler::resample_channels},
};

/// Strech the audio data using the given tempo map, not preserving the pitch.
/// The returned audio data will start at the beginning of the region.
pub fn tempo_strech(
    src_region: &AudioRegion,
    dst_buffer: &mut [f32],
    target_sample_rate: u64,
    target_channels: usize,
    tempo_map: &TempoMap,
) {
    let region_end = src_region.start + src_region.duration;
    let src_channels = src_region.channels as usize;

    // Create a section list by splitting the region into sections based on tempo change events
    // Get the first event on or before the region start beat
    let start_index = tempo_map
        .events
        .partition_point(|e| e.ticks() <= src_region.start)
        .saturating_sub(1);
    // Loop over the events until it surpasses the region end beat
    // (0: Start ticks, 1: End ticks, 2: BPM of the section)
    // Allocate a vector with a capacity of 16 for now,
    // assuming the number of tempo change in a region is not so large in common cases
    let mut sections: Vec<(Ticks, Ticks, f64)> = Vec::with_capacity(16);
    let mut i = start_index;

    while let Some(event) = tempo_map.events.get(i) {
        // Break if the event beat surpasses the region end beat
        if event.ticks() >= region_end {
            break;
        }

        // Get the start and the end beat of the section
        let section_start = if i == start_index {
            src_region.start
        } else {
            event.ticks()
        };
        let section_end = tempo_map
            .events
            .get(i + 1)
            .map(|next| next.ticks().min(region_end))
            .unwrap_or(region_end);

        // Push the section
        sections.push((section_start, section_end, event.bpm()));
        i += 1;
    }

    // Loop over the sections and resample the audio
    // `src_region.data` is indexed relative to the region's own start, not to the absolute project time,
    // so section ticks must be offset by the region's start sample
    let region_start_sample = tempo_map.ticks_to_samples(src_region.start);

    let mut output_data = Vec::new();

    // Resample each tempo section and append it to the output data
    for section in sections {
        // Calculate the relative start and the end index
        let src_start_sample = tempo_map
            .ticks_to_samples(section.0)
            .saturating_sub(region_start_sample)
            .min(src_region.frames);
        let src_end_sample = tempo_map
            .ticks_to_samples(section.1)
            .saturating_sub(region_start_sample)
            .min(src_region.frames);

        // Calculate the number of samples in the section and skip if it's zero
        let section_samples = src_end_sample - src_start_sample;
        if section_samples == 0 {
            continue;
        }

        // Calculate the start and end index of the section in the source buffer
        let src_start_index = src_start_sample * src_channels;
        let src_end_index = src_end_sample * src_channels;

        // Then get the section data from the source buffer
        let section_data = &src_region.data[src_start_index..src_end_index];

        // Calculate the source sample rate to change the tempo
        let src_sample_rate =
            (src_region.sample_rate as f64 * (src_region.base_bpm / section.2)) as u64;
        let resampled_data = resample_channels(
            section_data,
            src_channels,
            section_samples,
            src_sample_rate,
            target_sample_rate,
        );

        // Append the resampled audio to the output data
        output_data.extend(resampled_data);
    }

    let active_channels = target_channels.min(src_channels);

    // Finally add the output data to the output buffer while interleaving the channels
    for (dst_frame, src_frame) in dst_buffer
        .chunks_exact_mut(target_channels)
        .zip(output_data.chunks_exact(src_channels))
    {
        for ch in 0..active_channels {
            // Add the sample value
            dst_frame[ch] += src_frame[ch];
        }
    }
}
