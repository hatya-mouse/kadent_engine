const FP_TO_F32_SCALE: f32 = 1.0 / (1u64 << 32) as f32;

pub fn resample_channels(
    source: &[f32],
    source_samples: usize,
    source_sample_rate: u64,
    source_channels: usize,
    target_sample_rate: u64,
    target_channels: usize,
) -> Vec<f32> {
    // Calculate the ratio of the source and the target sample rate
    let ratio_fp = (source_sample_rate << 32) / target_sample_rate;
    let source_samples_fp = (source_samples as u64) << 32;
    let source_sample_rate_fp = source_sample_rate << 32;
    let target_sample_rate_fp = target_sample_rate << 32;

    // Calculate the length of the output array (interleaved) and fill it with zeros
    let full_len = target_channels
        * ((source_samples_fp * target_sample_rate_fp / source_sample_rate_fp) >> 32) as usize;
    let mut output = vec![0f32; full_len];

    for target_channel in 0..target_channels.min(source_channels) {
        let mut read_pos_fp = 0u64;
        let mut output_sample_count = target_channel;

        while read_pos_fp + (1u64 << 32) < source_samples_fp {
            // Calculate the index from the read position
            let index = (read_pos_fp >> 32) as usize;
            let remainder_fp = read_pos_fp & 0xFFFFFFFF;
            let remainder = remainder_fp as f32 * FP_TO_F32_SCALE;

            // Get the two samples to interpolate the sample
            let src_before = source[index * source_channels + target_channel];
            let src_after = source[(index + 1) * source_channels + target_channel];
            // Perform linear interpolation (Lerp)
            let interpolated_sample = src_before + remainder * (src_after - src_before);
            if let Some(sample) = output.get_mut(output_sample_count) {
                *sample = interpolated_sample;
            }

            // Increment the read position and sample count
            read_pos_fp += ratio_fp;
            output_sample_count += target_channels;
        }
    }

    output
}
