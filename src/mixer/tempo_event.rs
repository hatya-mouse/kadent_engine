use crate::data_types::Ticks;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

#[derive(Clone, Serialize, Deserialize)]
pub struct TempoEvent {
    pub tick: Ticks,
    pub bpm: f64,
    pub sample_offset: usize,
}

impl PartialEq for TempoEvent {
    fn eq(&self, other: &Self) -> bool {
        self.tick == other.tick
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
        if self.tick > other.tick {
            Ordering::Greater
        } else if self.tick == other.tick {
            Ordering::Equal
        } else {
            Ordering::Less
        }
    }
}
