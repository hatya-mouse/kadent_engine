/// Converts seconds to samples with the given sample rate.
#[inline]
pub fn seconds_to_samples(seconds: f64, sample_rate: u64) -> usize {
    (seconds * sample_rate as f64).round() as usize
}

/// Converts samples to seconds with the given sample rate.
#[inline]
pub fn samples_to_seconds(samples: usize, sample_rate: u64) -> f64 {
    samples as f64 / sample_rate as f64
}

/// Converts the number of samples based on a given ratio.
/// The ratio must be calculated with the following formula:
///
/// ```
/// ratio = output_sample_rate / input_sample_rate
/// ```
#[inline]
pub fn convert_rate_with_ratio(samples: usize, ratio: f64) -> usize {
    (samples as f64 * ratio).round() as usize
}
