use crate::track::{RegionID, note_track::NoteID};

#[derive(Debug, Clone)]
pub(super) struct VoiceEvent {
    pub(super) id: (RegionID, NoteID),
    pub sample_index: usize,
    pub pitch: f32,
    pub velocity: f32,
    pub is_note_on: bool,
}

impl VoiceEvent {
    pub fn new(
        id: (RegionID, NoteID),
        sample_index: usize,
        pitch: f32,
        velocity: f32,
        is_note_on: bool,
    ) -> Self {
        Self {
            id,
            sample_index,
            pitch,
            velocity,
            is_note_on,
        }
    }
}
