pub fn resample_channels(
    source: &[f32],
    source_samples: usize,
    source_sample_rate: u64,
    source_channels: usize,
    target_sample_rate: u64,
    target_channels: usize,
) -> Vec<f32> {
    if source_samples == 0 || target_sample_rate == 0 {
        return Vec::new();
    }

    // Calculate the length of the output array (interleaved) and fill it with zeros
    let target_samples = ((source_samples as u128 * target_sample_rate as u128)
        / source_sample_rate as u128) as usize;
    let full_len = target_samples * target_channels;
    let mut output = vec![0f32; full_len];

    // Calculate the ratio of the source and the target sample rate
    let ratio = source_sample_rate as f64 / target_sample_rate as f64;
    let active_channels = target_channels.min(source_channels);

    // Loop through each sample in the target array
    for sample in 0..target_samples {
        // Calculate the corresponding position in the source array
        let src_pos = sample as f64 * ratio;
        let index = src_pos as usize;
        let remainder = (src_pos - index as f64) as f32;

        if index + 1 >= source_samples {
            break;
        }

        // Get numbers to interpolate between
        let src_index_0 = index * source_channels;
        let src_index_1 = (index + 1) * source_channels;
        let dst_index = sample * target_channels;

        for channel in 0..active_channels {
            let src_0 = source[src_index_0 + channel];
            let src_1 = source[src_index_1 + channel];
            output[dst_index + channel] = src_0 + (src_1 - src_0) * remainder;
        }
    }

    output
}
