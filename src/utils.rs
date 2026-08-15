#[inline]
pub(crate) fn seconds_to_samples(seconds: f64, sample_rate: u64) -> usize {
    (seconds * sample_rate as f64).round() as usize
}

#[inline]
pub(crate) fn samples_to_seconds(samples: usize, sample_rate: u64) -> f64 {
    samples as f64 / sample_rate as f64
}

#[inline]
pub(crate) fn convert_rate(
    samples: usize,
    input_sample_rate: u64,
    output_sample_rate: u64,
) -> usize {
    seconds_to_samples(
        samples_to_seconds(samples, input_sample_rate),
        output_sample_rate,
    )
}

/// Converts the number of samples based on a given ratio.
/// The ratio must be calculated with the following formula:
///
/// ```
/// ratio = output_sample_rate / input_sample_rate
/// ```
#[inline]
pub(crate) fn convert_rate_with_ratio(samples: usize, ratio: f64) -> usize {
    (samples as f64 * ratio).round() as usize
}
