use crate::data_types::Ticks;
use serde::{Deserialize, Serialize};

/// Processed note generated from sequenced note data.
/// Should only be used for processing the note data in the `NoteTrack`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(super) struct ProcessedNote {
    /// Unique ID in the whole processed notes array.
    pub id: usize,
    /// Absolute start position in ticks.
    /// This is used to sort the notes in the `NoteTrack`.
    pub start: Ticks,
    /// Duration of the note in ticks.
    pub duration: Ticks,
    /// Frequency of the note.
    pub pitch: f32,
    /// Velocity of the note.
    pub velocity: f32,
}
