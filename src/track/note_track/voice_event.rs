use crate::data_types::MidiEvent;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Eq, Hash, PartialEq, Debug, Serialize, Deserialize)]
pub enum VoiceEventID {
    Midi { pitch: u8 },
    SequencedNote { id: usize },
}

#[derive(Debug, Clone)]
pub(super) enum VoiceEventKind {
    NoteOn { pitch: f32, velocity: f32 },
    NoteOff,
}

#[derive(Debug, Clone)]
pub(super) struct VoiceEvent {
    /// The time when the event is to be emitted in samples.
    pub sample_time: usize,
    /// The kind of the voice.
    pub kind: VoiceEventKind,
    /// The identifier of the voice. This should be used to match NoteOn with NoteOff.
    pub id: VoiceEventID,
}

impl VoiceEvent {
    pub fn new(sample_time: usize, kind: VoiceEventKind, id: VoiceEventID) -> Self {
        Self {
            sample_time,
            kind,
            id,
        }
    }

    pub fn from_midi_event(sample_time: usize, midi_event: MidiEvent) -> Self {
        match midi_event {
            MidiEvent::NoteOn { pitch, velocity } => Self {
                sample_time,
                kind: VoiceEventKind::NoteOn {
                    pitch: pitch as f32,
                    velocity: velocity as f32 / 256f32,
                },
                id: VoiceEventID::Midi { pitch },
            },
            MidiEvent::NoteOff { pitch } => Self {
                sample_time,
                kind: VoiceEventKind::NoteOff,
                id: VoiceEventID::Midi { pitch },
            },
        }
    }
}

// Implement Eq and Ord for BinaryHeap

impl PartialEq for VoiceEvent {
    fn eq(&self, other: &Self) -> bool {
        self.sample_time == other.sample_time
    }
}

impl Eq for VoiceEvent {}

impl PartialOrd for VoiceEvent {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for VoiceEvent {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        if self.sample_time < other.sample_time {
            std::cmp::Ordering::Less
        } else if self.sample_time == other.sample_time {
            std::cmp::Ordering::Equal
        } else {
            std::cmp::Ordering::Greater
        }
    }
}
