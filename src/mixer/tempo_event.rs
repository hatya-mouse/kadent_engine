use crate::data_types::Ticks;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

#[derive(Clone, Serialize, Deserialize)]
pub struct TempoEvent {
    pub(super) ticks: Ticks,
    pub(super) bpm: f64,
    pub(super) sample_offset: usize,
    /// Cached factor for converting ticks to samples, calculated from the audio context.
    samples_per_tick_fp: u64,
}

impl TempoEvent {
    pub fn new(ticks: Ticks, bpm: f64, sample_offset: usize) -> Self {
        Self {
            ticks,
            bpm,
            sample_offset,
            samples_per_tick_fp: 0,
        }
    }

    pub fn ticks(&self) -> Ticks {
        self.ticks
    }

    pub fn bpm(&self) -> f64 {
        self.bpm
    }

    pub fn sample_offset(&self) -> usize {
        self.sample_offset
    }

    pub fn set_ticks(&mut self, ticks: Ticks) {
        self.ticks = ticks;
    }

    pub fn set_bpm(&mut self, bpm: f64, sample_rate: u64, resolution: u64) {
        self.update_factor(sample_rate, resolution);
        self.bpm = bpm;
    }

    pub fn set_sample_offset(&mut self, sample_offset: usize) {
        self.sample_offset = sample_offset;
    }

    pub(super) fn update_factor(&mut self, sample_rate: u64, resolution: u64) {
        let scale = 1 << 32;
        self.samples_per_tick_fp = ((60u128 * sample_rate as u128 * scale)
            / (resolution as u128 * self.bpm as u128)) as u64;
    }

    /// Converts the given ticks to samples using this `TempoEvent`'s bpm.
    #[inline]
    pub(super) fn ticks_to_samples(&self, target_ticks: Ticks) -> usize {
        if target_ticks.0 <= self.ticks.0 {
            return self.sample_offset;
        }

        let remaining_ticks = target_ticks.0 - self.ticks.0;
        let remaining_samples =
            ((remaining_ticks as u128 * self.samples_per_tick_fp as u128) >> 32) as usize;
        self.sample_offset + remaining_samples
    }
}

impl PartialEq for TempoEvent {
    fn eq(&self, other: &Self) -> bool {
        self.ticks == other.ticks
    }
}

impl Eq for TempoEvent {}

impl PartialOrd for TempoEvent {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for TempoEvent {
    fn cmp(&self, other: &Self) -> Ordering {
        if self.ticks > other.ticks {
            Ordering::Greater
        } else if self.ticks == other.ticks {
            Ordering::Equal
        } else {
            Ordering::Less
        }
    }
}
