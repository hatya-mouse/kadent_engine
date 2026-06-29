use crate::{data_types::Ticks, track::note_track::NoteModifierID};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Clone, Copy, Default, Eq, Hash, PartialEq, Debug, Serialize, Deserialize)]
pub struct NoteID(pub u64);

#[derive(Clone, Serialize, Deserialize)]
pub struct Note {
    /// Relative start position in the region in ticks.
    pub start: Ticks,
    /// Duration of the note in ticks.
    pub duration: Ticks,
    /// Frequency of the note.
    pub pitch: f32,
    /// Velocity of the note.
    pub velocity: f32,
    /// IDs of the applied modifiers for the note.
    /// Modifiers will be applied in the order set in the `NoteTrack`.
    pub modifiers: HashSet<NoteModifierID>,
}

impl Note {
    pub fn new(start: Ticks, duration: Ticks, pitch: f32, velocity: f32) -> Self {
        Self {
            start,
            duration,
            pitch,
            velocity,
            modifiers: HashSet::new(),
        }
    }
}
