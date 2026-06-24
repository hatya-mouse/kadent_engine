use crate::{
    data_types::{AudioContext, Ticks},
    mixer::TempoEvent,
};

#[derive(Clone, Default)]
pub struct TempoMap {
    /// The tempo events in the tempo map, sorted by their ticks.
    pub(super) events: Vec<TempoEvent>,
    audio_ctx: AudioContext,
}

impl TempoMap {
    // --- NEW ---

    /// Creates a new TempoMap.
    pub fn new(audio_ctx: AudioContext, initial_bpm: f64) -> Self {
        Self {
            events: vec![TempoEvent::new(Ticks(0), initial_bpm, 0, &audio_ctx)],
            audio_ctx,
        }
    }

    // --- AUDIO CONTEXT ---

    /// Sets the audio context to the new one and calculate the sample offsets of all events in the TempoMap.
    pub fn set_audio_ctx(&mut self, audio_ctx: AudioContext) {
        self.audio_ctx = audio_ctx;
        // Calculate the offsets of all events
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
        for i in after_index..self.events.len() {
            if i == 0 {
                self.events[i].sample_offset = 0;
            } else {
                let prev = &self.events[i - 1];
                let tick_diff = (self.events[i].ticks.0 - prev.ticks.0) as u128;
                // Calculate as u128 to avoid wrapping around to avoid calculation error
                // when the ticks difference is large enough
                let samples = (60u128 * tick_diff * self.audio_ctx.sample_rate as u128)
                    / (self.audio_ctx.resolution as u128 * prev.bpm as u128);
                self.events[i].sample_offset = prev.sample_offset + samples as usize;
            }
        }
    }

    // --- TICKS CONVERSION ---

    /// Convert the Ticks to sampels using the tempo map.
    pub fn ticks_to_samples(&self, ticks: Ticks) -> usize {
        let idx = self.events.partition_point(|e| e.ticks <= ticks) - 1;
        self.events[idx].ticks_to_samples(ticks)
    }

    /// Converts samples to Ticks using the tempo map.
    pub fn samples_to_ticks(&self, samples: usize) -> Ticks {
        // Find the last event before the sample
        let idx = self
            .events
            .partition_point(|e| e.sample_offset <= samples)
            .saturating_sub(1);
        let event = &self.events[idx];

        // Calculate the elapsed samples from the event's sample offset
        let elapsed_samples = samples - event.sample_offset;
        // Convert the elapsed samples to ticks
        let elapsed_ticks =
            (elapsed_samples as u128 * self.audio_ctx.resolution as u128 * event.bpm as u128)
                / (60u128 * self.audio_ctx.sample_rate as u128);

        event.ticks + Ticks(elapsed_ticks as u64)
    }
}
