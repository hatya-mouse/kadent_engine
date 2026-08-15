use crate::{data_types::Ticks, timing::TempoEvent};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct TempoMap {
    /// The tempo events in the tempo map, sorted by their ticks.
    pub events: Vec<TempoEvent>,
    /// The initial BPM of the tempo map, used before the first tempo event.
    pub initial_bpm: f64,
    /// The resolution of the audio context, used for converting between ticks and seconds.
    pub resolution: u64,
}

impl TempoMap {
    // --- NEW ---

    /// Creates a new TempoMap.
    pub fn new(resolution: u64, initial_bpm: f64) -> Self {
        let mut map = Self {
            events: vec![],
            initial_bpm,
            resolution,
        };
        map.bake();
        map
    }

    // --- AUDIO CONTEXT ---

    /// Sets the resolution to the given one.
    pub fn set_resolution(&mut self, resolution: u64) {
        self.resolution = resolution;
        self.bake();
    }

    // --- TEMPO EVENT MANAGEMENT ---

    /// Adds a new tempo event to the tempo map, preserving the order of events.
    pub fn add_event(&mut self, event: TempoEvent) {
        // Insert the event while preserving the order
        match self.events.binary_search(&event) {
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
        self.bake();
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
            self.bake();
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
        self.bake();
    }

    // --- OFFSET CALCULATION ---

    /// Recalculates the seconds for the tempo events.
    fn bake(&mut self) {
        self.events.sort_by_key(|e| e.tick.0);
        let mut current_time = 0.0;
        let mut prev_tick = 0i64;
        let mut prev_bpm = self.initial_bpm;

        for event in self.events.iter_mut() {
            // Calculate the seconds from the previous event to the current event
            let delta_ticks = event.tick.0.saturating_sub(prev_tick);
            let seconds_per_tick = 60.0 / (prev_bpm * self.resolution as f64);
            current_time += delta_ticks as f64 * seconds_per_tick;

            // Update the event's time_seconds and the previous tick and bpm for the next iteration
            event.time_seconds = current_time;
            prev_tick = event.tick.0;
            prev_bpm = event.bpm;
        }
    }

    // --- TICKS CONVERSION ---

    /// Converts the Ticks to seconds using the tempo map.
    pub fn ticks_to_seconds(&self, tick: Ticks) -> f64 {
        let idx = self.events.partition_point(|e| e.tick.0 <= tick.0);

        if idx == 0 {
            // The tick is before the first event
            let seconds_per_tick = self.seconds_per_tick(self.initial_bpm);
            tick.0 as f64 * seconds_per_tick
        } else {
            // Get the last event before the tick and calculate seconds from the event to the tick
            let event = &self.events[idx - 1];
            let delta_ticks = tick.0 - event.tick.0;
            let seconds_per_tick = self.seconds_per_tick(event.bpm);
            // Then add the calculated seconds to the event's time_seconds
            event.time_seconds + (delta_ticks as f64 * seconds_per_tick)
        }
    }

    /// Converts the seconds to Ticks using the tempo map.
    pub fn seconds_to_ticks(&self, seconds: f64) -> Ticks {
        let idx = self.events.partition_point(|e| e.time_seconds <= seconds);

        if idx == 0 {
            // The tick is before the first event
            let ticks_per_second = self.ticks_per_second(self.initial_bpm);
            Ticks((seconds * ticks_per_second) as i64)
        } else {
            // Get the last event before the tick and calculate seconds from the event to the tick
            let event = &self.events[idx - 1];
            let delta_seconds = seconds - event.time_seconds;
            let ticks_per_second = self.ticks_per_second(event.bpm);
            // Add the calculated ticks to the event's tick
            Ticks(event.tick.0 + (delta_seconds * ticks_per_second) as i64)
        }
    }

    #[inline]
    fn seconds_per_tick(&self, bpm: f64) -> f64 {
        60.0 / (bpm * self.resolution as f64)
    }

    #[inline]
    fn ticks_per_second(&self, bpm: f64) -> f64 {
        bpm * self.resolution as f64 / 60.0
    }
}
