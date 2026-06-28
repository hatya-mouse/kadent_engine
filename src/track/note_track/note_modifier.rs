use crate::track::note_track::Note;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Copy, Default, Eq, Hash, PartialEq, Debug, Serialize, Deserialize)]
pub struct NoteModifierID(Uuid);

pub trait NoteModifier: Send {
    /// Clones the modifier.
    fn clone_box(&self) -> Box<dyn NoteModifier>;

    /// Processes the Note using the modifier.
    fn process(&mut self, input_notes: &[Note]) -> Vec<Note>;
}

impl Clone for Box<dyn NoteModifier> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}
