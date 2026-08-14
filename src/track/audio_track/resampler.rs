use rayon::{
    iter::{IndexedParallelIterator, ParallelIterator},
    slice::ParallelSliceMut,
};

pub fn resample_channels(source: &[f32], channels: usize, ratio: f64) -> Vec<f32> {
    if source.is_empty() || channels == 0 || ratio == 0.0 {
        return Vec::new();
    }
    let src_samples = source.len() / channels;

    // Calculate the length of the output array (interleaved) and fill it with zeros
    let target_samples = (src_samples as f64 * ratio).ceil() as usize;
    let full_len = target_samples * channels;
    let mut output = vec![0f32; full_len];

    // Loop through each sample in the target array
    output
        .par_chunks_exact_mut(channels)
        .enumerate()
        .for_each(|(sample, sample_buffer)| {
            // Calculate the corresponding position in the source array
            let src_pos = sample as f64 * ratio;
            let index = src_pos as usize;

            if index + 1 < src_samples {
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
