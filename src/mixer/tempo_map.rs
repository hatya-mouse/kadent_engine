use crate::{
    data_types::{AudioContext, PlaybackContext, Ticks},
    mixer::{TempoEvent, TempoSection},
    track::audio_track::AudioDataInfo,
};

#[derive(Clone, Default)]
pub struct TempoMap {
    /// The tempo events in the tempo map, sorted by their ticks.
    pub events: Vec<TempoEvent>,
    audio_ctx: AudioContext,
    playback_ctx: Option<PlaybackContext>,
}

impl TempoMap {
    // --- NEW ---

    /// Creates a new TempoMap.
    pub fn new(audio_ctx: AudioContext, initial_bpm: f64) -> Self {
        Self {
            events: vec![TempoEvent::new(Ticks(0), initial_bpm, 0)],
            audio_ctx,
            playback_ctx: None,
        }
    }

    // --- AUDIO CONTEXT ---

    /// Sets the audio context to the new one and calculate the sample offsets of all events in the TempoMap.
    pub fn set_audio_ctx(&mut self, audio_ctx: AudioContext) {
        self.audio_ctx = audio_ctx;
        self.calculate_sample_offsets(0);
    }

    // --- PREPARATION ---

    /// Sets the playback context to the new one and calculate the sample offsets of all events in the TempoMap.
    pub fn prepare(&mut self, playback_ctx: PlaybackContext) {
        self.playback_ctx = Some(playback_ctx);
        self.calculate_sample_offsets(0);
    }

    // --- TEMPO EVENT MANAGEMENT ---

    /// Adds a new tempo event to the tempo map, preserving the order of events.
    pub fn add_event(&mut self, event: TempoEvent) {
        // Insert the event while preserving the order
        let index = match self.events.binary_search(&event) {
            Ok(index) => {
                // Overwrite the old event if the event with the same beat exists
                self.events[index] = event;
                index
            }
            Err(index) => {
                self.events.insert(index, event);
                index
            }
        };

        // Calculate the sample offsets of the events after the modified index
        self.calculate_sample_offsets(index);
    }

    /// Removes the tempo event from the tempo map.
    pub fn remove_event(&mut self, index: usize) {
        // Cannot return the event with the zero beats
        if index == 0 {
            return;
        }

        // Ensure that the TempoMap has at least one event
        if self.events.len() > 1 {
            // Remove the event in the index
            self.events.remove(index);
            // Calculate the sample offsets of the events after the removed index
            self.calculate_sample_offsets(index);
        }
    }

    /// Change the BPM of the event in the specified index.
    pub fn change_bpm(&mut self, index: usize, bpm: f64) {
        // Get a mutable reference to the target event
        let Some(event) = self.events.get_mut(index) else {
            return;
        };

        // Update the bpm
        event.bpm = bpm;

        // Calculate the sample offsets of the events after the event
        self.calculate_sample_offsets(index);
    }

    // --- OFFSET CALCULATION ---

    /// Recalculates the offsets of the events after the given index,
    /// storing the results in the events vector.
    fn calculate_sample_offsets(&mut self, after_index: usize) {
        let Some(sample_rate) = self.playback_ctx.as_ref().map(|ctx| ctx.sample_rate) else {
            return;
        };
        let resolution = self.audio_ctx.resolution;
        for i in after_index..self.events.len() {
            if i == 0 {
                self.events[i].sample_offset = 0;
            } else {
                let prev = &self.events[i - 1];
                let tick_diff = (self.events[i].ticks.0 - prev.ticks.0) as u128;
                // Calculate as u128 to avoid wrapping around to avoid calculation error
                // when the ticks difference is large enough
                let samples = (60u128 * tick_diff * sample_rate as u128)
                    / (self.audio_ctx.resolution as u128 * prev.bpm as u128);
                self.events[i].sample_offset = prev.sample_offset + samples as usize;
            }
            // Sync the fixed-point factor used by ticks_to_samples
            self.events[i].update_factor(sample_rate, resolution);
        }
    }

    // --- TICKS CONVERSION ---

    /// Convert the Ticks to sampels using the tempo map.
    pub fn ticks_to_samples(&self, ticks: Ticks) -> usize {
        debug_assert!(
            self.events.is_empty() || self.events[0].ticks.0 == 0,
            "The first tempo event must be at tick 0"
        );
        if self.events.is_empty() || ticks.0 < self.events[0].ticks.0 {
            return 0;
        }

        let idx = self
            .events
            .partition_point(|e| e.ticks <= ticks)
            .saturating_sub(1);
        self.events[idx].ticks_to_samples(ticks)
    }

    /// Converts samples to Ticks using the tempo map.
    pub fn samples_to_ticks(&self, samples: usize) -> Ticks {
        let Some(sample_rate) = self.playback_ctx.as_ref().map(|ctx| ctx.sample_rate) else {
            return Ticks(0);
        };
        // Find the last event before the sample
        let idx = self
            .events
            .partition_point(|e| e.sample_offset <= samples)
            .saturating_sub(1);
        let event = &self.events[idx];

        // Calculate the elapsed samples from the event's sample offset
        let elapsed_samples = samples - event.sample_offset;
        // Convert the elapsed samples to ticks
        let elapsed_ticks = (elapsed_samples as f64 * self.audio_ctx.resolution as f64 * event.bpm)
            / (60f64 * sample_rate as f64);
        event.ticks + Ticks(elapsed_ticks.round() as i64)
    }

    // --- SECTION CALCULATION ---

    /// Calculates the number of samples from the start of the audio data to the target sample in the global sample rate.
    ///
    /// # Parameters
    /// - `target_sample`: The global sample index to convert.
    /// - `src_info`: Information of the audio data that you want to resample.
    pub(crate) fn global_to_local_sample(
        &self,
        data_start_sample: usize,
        target_sample: usize,
        src_info: &AudioDataInfo,
    ) -> usize {
        if target_sample <= data_start_sample {
            return 0;
        }

        let sample_rate = match &self.playback_ctx {
            Some(ctx) => ctx.sample_rate,
            None => return 0,
        };

        // Calculate where to start iterating over the tempo events
        let start_index = self
            .events
            .partition_point(|e| e.sample_offset <= data_start_sample)
            .saturating_sub(1);
        let mut current_sample_global = data_start_sample;
        let mut current_sample_local = 0;

        for (i, event) in self.events.iter().enumerate().skip(start_index) {
            let resample_ratio =
                (sample_rate as f64 * event.bpm()) / (src_info.sample_rate as f64 * src_info.bpm);
            let next_event_sample = self
                .events
                .get(i + 1)
                .map(|event| event.sample_offset())
                .unwrap_or(usize::MAX);

            // Calculate the number of samples in local sample rate in the section
            let section_end_global = next_event_sample.min(target_sample);
            let section_samples_global = section_end_global - current_sample_global;
            let section_samples_local = (section_samples_global as f64 / resample_ratio) as usize;
            current_sample_local += section_samples_local;

            // Break if the section end is equal to the target sample
            if section_end_global == target_sample {
                break;
            }

            current_sample_global += section_samples_global;
        }

        current_sample_local
    }

    /// Returns a vector of TempoSection that holds the tempo and the resample ratio in it.
    ///
    /// # Parameters
    /// - `start_sample`: The start sample index of the range in the global sample rate.
    /// - `end_sample`: The end sample index of the range in the global sample rate.
    /// - `src_info`: Information of the audio data that you want to resample.
    pub(crate) fn get_sections_in_range(
        &self,
        start_sample: usize,
        end_sample: usize,
        src_info: &AudioDataInfo,
    ) -> Vec<TempoSection> {
        let mut sections = Vec::new();
        if start_sample >= end_sample || self.events.is_empty() {
            return sections;
        }

        let sample_rate = match &self.playback_ctx {
            Some(ctx) => ctx.sample_rate,
            None => return sections,
        };

        // Calculate where to start iterating over the tempo events
        let start_index = self
            .events
            .partition_point(|e| e.sample_offset <= start_sample)
            .saturating_sub(1);
        let mut current_sample = start_sample;
        let mut current_local_sample = 0;

        // Loop over the tempo change events and create sections based on the tempo changes
        for (i, event) in self.events.iter().enumerate().skip(start_index) {
            let next_event_sample = self
                .events
                .get(i + 1)
                .map(|event| event.sample_offset())
                .unwrap_or(usize::MAX);

            // Clamp the section end to the end_sample to avoid going beyond the requested range
            let section_end = next_event_sample.min(end_sample);
            // Calculate the number of local samples in the section

            if current_sample < section_end {
                // Calculate the resample ratio for the current section
                let resample_ratio = (sample_rate as f64 * event.bpm())
                    / (src_info.sample_rate as f64 * src_info.bpm);
                let local_samples = (section_end - current_sample) as f64 / resample_ratio;

                // Get the current local sample index for the section
                let local_start_sample = current_local_sample;
                let local_end_sample = current_local_sample + local_samples as usize;
                current_local_sample = local_end_sample;

                sections.push(TempoSection::new(
                    current_sample,
                    section_end,
                    local_start_sample,
                    local_end_sample,
                    resample_ratio,
                ));
                current_sample = section_end;
            }

            if current_sample >= end_sample {
                break;
            }
        }

        sections
    }
}
