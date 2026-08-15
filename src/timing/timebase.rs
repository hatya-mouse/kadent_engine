use crate::{data_types::Ticks, timing::TempoMap, utils::seconds_to_samples};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Timebase {
    /// Tick / beat based timing
    Musical,
    /// Actual-time based timing
    Time,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum SeekPosition {
    Musical(Ticks),
    Time(f64),
}

impl SeekPosition {
    pub fn to_sample(&self, tempo_map: &TempoMap, sample_rate: u64) -> usize {
        match *self {
            SeekPosition::Musical(ticks) => tempo_map.ticks_to_samples(ticks, sample_rate),
            SeekPosition::Time(seconds) => seconds_to_samples(seconds, sample_rate),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum TimeBounds {
    Musical {
        start: Ticks,
        duration: Ticks,
    },
    Time {
        start_seconds: f64,
        duration_seconds: f64,
    },
}

impl TimeBounds {
    // --- TICK CALCULATION ---

    /// Calculates the start and end ticks of the region from the given tempo map.
    pub fn tick_range(&self, tempo_map: &TempoMap) -> (Ticks, Ticks) {
        (self.start_tick(tempo_map), self.end_tick(tempo_map))
    }

    /// Calculates the start tick of the region from the given tempo map.
    pub fn start_tick(&self, tempo_map: &TempoMap) -> Ticks {
        match *self {
            TimeBounds::Musical { start, .. } => start,
            TimeBounds::Time { start_seconds, .. } => tempo_map.seconds_to_ticks(start_seconds),
        }
    }

    /// Calculates the end tick of the region from the given tempo map.
    pub fn end_tick(&self, tempo_map: &TempoMap) -> Ticks {
        match *self {
            TimeBounds::Musical { start, duration } => start + duration,
            TimeBounds::Time {
                start_seconds,
                duration_seconds,
            } => tempo_map.seconds_to_ticks(start_seconds + duration_seconds),
        }
    }

    // --- SECONDS CALCULATION ---

    /// Calculates the start seconds of the region from the given tempo map.
    pub fn start_seconds(&self, tempo_map: &TempoMap) -> f64 {
        match *self {
            TimeBounds::Musical { start, .. } => tempo_map.ticks_to_seconds(start),
            TimeBounds::Time { start_seconds, .. } => start_seconds,
        }
    }

    /// Calculates the end tick of the region from the given tempo map.
    pub fn end_seconds(&self, tempo_map: &TempoMap) -> f64 {
        match *self {
            TimeBounds::Musical { start, duration } => tempo_map.ticks_to_seconds(start + duration),
            TimeBounds::Time {
                start_seconds,
                duration_seconds,
            } => start_seconds + duration_seconds,
        }
    }

    // --- SAMPLE CONVERSION ---

    // Calculate the start and end samples of the region from the given tempo map and sample rate.
    pub(crate) fn sample_range(&self, tempo_map: &TempoMap, sample_rate: u64) -> (usize, usize) {
        (
            self.start_sample(tempo_map, sample_rate),
            self.end_sample(tempo_map, sample_rate),
        )
    }

    // Calculates the start sample of the region from the given tempo map and sample rate.
    pub(crate) fn start_sample(&self, tempo_map: &TempoMap, sample_rate: u64) -> usize {
        let start_seconds = self.start_seconds(tempo_map);
        seconds_to_samples(start_seconds, sample_rate)
    }

    // Calculates the end sample of the region from the given tempo map and sample rate.
    pub(crate) fn end_sample(&self, tempo_map: &TempoMap, sample_rate: u64) -> usize {
        let end_seconds = self.end_seconds(tempo_map);
        seconds_to_samples(end_seconds, sample_rate)
    }
}
