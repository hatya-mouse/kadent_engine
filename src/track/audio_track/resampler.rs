use rayon::{
    iter::{IndexedParallelIterator, ParallelIterator},
    slice::ParallelSliceMut,
};

pub fn resample_channels(
    source: &[f32],
    channels: usize,
    source_samples: usize,
    source_sample_rate: u64,
    target_sample_rate: u64,
) -> Vec<f32> {
    if source_samples == 0 || target_sample_rate == 0 {
        return Vec::new();
    }

    // Calculate the length of the output array (interleaved) and fill it with zeros
    let target_samples = ((source_samples as u128 * target_sample_rate as u128)
        / source_sample_rate as u128) as usize;
    let full_len = target_samples * channels;
    let mut output = vec![0f32; full_len];

    // Calculate the ratio of the source and the target sample rate
    let ratio = source_sample_rate as f64 / target_sample_rate as f64;

    // Loop through each sample in the target array
    output
        .par_chunks_exact_mut(channels)
        .enumerate()
        .for_each(|(sample, sample_buffer)| {
            // Calculate the corresponding position in the source array
            let src_pos = sample as f64 * ratio;
            let index = src_pos as usize;

            if index + 1 < source_samples {
                let gradient = (src_pos - index as f64) as f32;

                // Get numbers to interpolate between
                let src_index_0 = index * channels;
                let src_index_1 = (index + 1) * channels;

                for channel in 0..channels {
                    let src_0 = source[src_index_0 + channel];
                    let src_1 = source[src_index_1 + channel];
                    sample_buffer[channel] = src_0 + (src_1 - src_0) * gradient;
                }
            }
        });

    output
}
