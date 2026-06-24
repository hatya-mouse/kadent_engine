use crate::data_types::Ticks;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Default, Eq, Hash, PartialEq, Debug, Serialize, Deserialize)]
pub struct NoteID(pub usize);

#[derive(Clone, Serialize, Deserialize)]
pub struct Note {
    /// Relative start position in the region in beats.
    pub start: Ticks,
    /// Duration of the note in beats.
    pub duration: Ticks,
    /// Frequency of the note.
    pub pitch: f32,
    /// Velocity of the note.
    pub velocity: f32,
}

impl Note {
    pub fn new(start: Ticks, duration: Ticks, pitch: f32, velocity: f32) -> Self {
        Self {
            start,
            duration,
            pitch,
            velocity,
        }
    }
}
