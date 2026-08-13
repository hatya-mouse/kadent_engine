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
/// - `start_sample`: The start sample index of the region to be streched.
/// - `end_sample`: The end sample index of the region to be streched.
/// - `dst_sample_rate`: The sample rate of the destination audio data.
/// - `tempo_map`: The tempo map to be used for streching the audio data.
pub(super) fn tempo_strech(
    src: &[f32],
    src_info: &AudioDataInfo,
    start_sample: usize,
    end_sample: usize,
    dst_sample_rate: u64,
    tempo_map: &TempoMap,
) -> Vec<f32> {
    // If the destination sample rate is zero, return immediately with an empty vector
    if src.is_empty() || dst_sample_rate == 0 {
        return Vec::new();
    }

    // Get the tempo sections in the given range from the tempo map
    let sections = tempo_map.get_sections_in_range(start_sample, end_sample, src_info);

    // Resample each tempo section and append it to the output data
    let mut output_data = Vec::new();

    for section in sections {
        // Number of samples in the section
        let section_samples = section
            .global_end_sample
            .saturating_sub(section.global_start_sample);
        if section_samples == 0 {
            continue;
        }

        // Get the start and end indices of the section in the source buffer
        let src_start_index = section.local_start_sample * src_info.channels;
        let src_end_index = section.local_end_sample * src_info.channels;

        // Then get the section data from the source buffer
        let section_data = &src.get(src_start_index..src_end_index).unwrap_or(&[]);

        if section.resample_ratio == 1.0 {
            // If the ratio is 1.0, we can skip resampling and just append the section data to the output data
            output_data.extend_from_slice(section_data);
        } else {
            // Calculate the ratio of the sample rate and the bpm
            let resampled_data =
                resample_channels(section_data, src_info.channels, section.resample_ratio);
            // Append the resampled audio to the output data
            output_data.extend(resampled_data);
        }
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
