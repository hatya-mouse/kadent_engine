use crate::{
    data_types::Ticks,
    mixer::TempoMap,
    track::audio_track::{AudioRegion, resampler::resample_channels},
};

/// Strech the audio data using the given tempo map, not preserving the pitch.
pub fn tempo_strech(
    src_region: &AudioRegion,
    target_sample_rate: usize,
    target_channels: usize,
    tempo_map: &TempoMap,
) -> Vec<f32> {
    let region_end = src_region.start + src_region.duration;

    // Create a section list by splitting the region into sections based on tempo change events
    // Get the first event on or before the region start beat
    let start_index = tempo_map
        .events
        .partition_point(|e| e.ticks <= src_region.start)
        .saturating_sub(1);
    // Loop over the events until it surpasses the region end beat
    // (0: Start ticks, 1: End ticks, 2: BPM of the section)
    let mut sections: Vec<(Ticks, Ticks, f64)> = Vec::new();
    let mut i = start_index;
    while let Some(event) = tempo_map.events.get(i) {
        // Break if the event beat surpasses the region end beat
        if event.ticks >= region_end {
            break;
        }

        // Get the start and the end beat of the section
        let section_start = if i == start_index {
            src_region.start
        } else {
            event.ticks
        };
        let section_end = tempo_map
            .events
            .get(i + 1)
            .map(|next| next.ticks.min(region_end))
            .unwrap_or(region_end);

        // Push the section
        sections.push((section_start, section_end, event.bpm));

        i += 1;
    }

    // Loop over the sections and resample the audio
    let mut output_data = Vec::new();
    for section in sections {
        // Calculate the relative start and the end index
        let src_start_sample = tempo_map.ticks_to_samples(section.0).min(src_region.frames);
        let src_end_sample = tempo_map.ticks_to_samples(section.1).min(src_region.frames);

        let src_start_index = src_start_sample * src_region.channels as usize;
        let src_end_index = src_end_sample * src_region.channels as usize;

        // Get the slice from the data
        let section_data = &src_region.data[src_start_index..src_end_index];
        let section_frames = src_end_sample - src_start_sample;

        // Calculate the source sample rate to change the tempo
        let src_sample_rate =
            (src_region.sample_rate as f64 * (src_region.base_bpm / section.2)) as usize;
        let resampled_data = resample_channels(
            section_data,
            section_frames,
            src_sample_rate,
            src_region.channels as usize,
            target_sample_rate,
            target_channels,
        );

        // Append the resampled audio to the output data
        output_data.extend(resampled_data);
    }

    output_data
}
