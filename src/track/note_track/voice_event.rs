use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Default, Eq, Hash, PartialEq, Debug, Serialize, Deserialize)]
pub struct VoiceEventID(pub u64);

#[derive(Debug, Clone)]
pub(super) struct VoiceEvent {
    /// The time when the event is to be emitted in samples.
    pub sample_time: usize,
    /// The kind of the voice.
    pub kind: VoiceEventKind,
}

#[derive(Debug, Clone)]
pub(super) enum VoiceEventKind {
    NoteOn { pitch: f32, velocity: f32 },
    NoteOff,
}

impl VoiceEvent {
    pub fn new(sample_time: usize, kind: VoiceEventKind) -> Self {
        Self { sample_time, kind }
    }
}

// Implement Eq and Ord for BinaryHeap

impl PartialEq for VoiceEvent {
    fn eq(&self, other: &Self) -> bool {
        &self.sample_time == &other.sample_time
    }
}

impl PartialOrd for VoiceEvent {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        if &self.sample_time < &other.sample_time {
            Some(std::cmp::Ordering::Less)
        } else if &self.sample_time == &other.sample_time {
            Some(std::cmp::Ordering::Equal)
        } else {
            Some(std::cmp::Ordering::Greater)
        }
    }
}

impl Eq for VoiceEvent {}

impl Ord for VoiceEvent {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.partial_cmp(other).unwrap()
    }
}
