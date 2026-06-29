#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Voice {
    /// A MIDI note number in f32.
    pub pitch: f32,
    /// The velocity of the voice.
    pub velocity: f32,
    pub age: f32,
    /// Whether the voice is active or not.
    pub is_active: bool,
    /// The gain of the note, primarily used to fade in or out the note to reduce pop noise.
    pub gain: f32,
}

impl Default for Voice {
    fn default() -> Self {
        Self {
            pitch: 0.0,
            velocity: 0.0,
            age: 0.0,
            is_active: false,
            gain: 1.0,
        }
    }
}

impl Voice {
    pub fn new(pitch: f32, velocity: f32, age: f32, is_active: bool) -> Self {
        Self {
            pitch,
            velocity,
            age,
            is_active,
            gain: 1.0,
        }
    }
}
