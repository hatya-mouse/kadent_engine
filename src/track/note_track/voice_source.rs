/// Whether the voice is generated from a MIDI input or a sequenced note.
/// This determines whether the voice's age should be incremented when paused.
#[derive(Clone, PartialEq, Eq)]
pub(super) enum VoiceSource {
    RealtimeMidi,
    SequencedNote,
}
