/// Represents a single section which is separated by tempo changes.
pub(crate) struct TempoSection {
    /// The start sample index of the section in the global sample rate and tempo.
    pub global_start_sample: usize,
    /// The end sample index of the section in the global sample rate and tempo.
    pub global_end_sample: usize,
    /// The relative start sample index of the section in the local sample rate.
    pub local_start_sample: usize,
    /// The relative end sample index of the section in the local sample rate.
    pub local_end_sample: usize,
    /// Resampling rate associated with this section, calculated by the following formula.
    ///
    /// ```
    /// (Source Sample Rate * Source BPM) / (Target Sample Rate * Target BPM)
    /// ```
    pub resample_ratio: f64,
}

impl TempoSection {
    pub(crate) fn new(
        global_start_sample: usize,
        global_end_sample: usize,
        local_start_sample: usize,
        local_end_sample: usize,
        resample_ratio: f64,
    ) -> Self {
        Self {
            global_start_sample,
            global_end_sample,
            local_start_sample,
            local_end_sample,
            resample_ratio,
        }
    }
}
