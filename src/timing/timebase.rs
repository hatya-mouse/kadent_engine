use crate::data_types::Ticks;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Timebase {
    /// Tick / beat based timing
    Musical,
    /// Actual-time based timing
    Time,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum RegionBounds {
    Musical {
        start: Ticks,
        duration: Ticks,
    },
    Time {
        start_seconds: f64,
        duration_seconds: f64,
    },
}
