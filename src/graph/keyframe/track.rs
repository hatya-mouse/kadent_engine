use crate::data_types::Ticks;

/// A track that stores keyframes for a specific node and input index.
pub struct KeyframeTrack {
    /// The vector of keyframes.
    keyframes: Vec<Keyframe>,
}

pub enum InterpolatableValue {
    Float(f32),
    Int(f32),
    Bool(bool),
}

pub struct Keyframe {
    pub ticks: Ticks,
    pub value: InterpolatableValue,
}

impl InterpolatableValue {
    pub fn to_ne_bytes(&self) -> Vec<u8> {
        match self {
            InterpolatableValue::Float(f) => f.to_ne_bytes().to_vec(),
            InterpolatableValue::Int(i) => i.to_ne_bytes().to_vec(),
            InterpolatableValue::Bool(b) => vec![if *b { 1u8 } else { 0u8 }],
        }
    }
}
