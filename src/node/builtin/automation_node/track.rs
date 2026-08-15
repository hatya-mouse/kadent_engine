use crate::{
    data_types::{PlaybackContext, Ticks},
    node::builtin::automation_node::{
        CurveType, constant::ConstantAutomationCursor, float::FloatAutomationCursor,
    },
    timing::TempoMap,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Keyframe<T> {
    pub ticks: Ticks,
    pub curve: CurveType,
    pub value: T,
}

/// A track that stores keyframes for a specific node and input index.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AutomationTrack {
    /// Float keyframe track, used for continuous values.
    /// The value is interpolated between keyframes based on the gradient calculated during preparation.
    Float {
        /// The vector of keyframes, sorted by ticks.
        keyframes: Vec<Keyframe<f32>>,
        /// Cached sample indices of the keyframes, sorted in the order of the keyframe index.
        #[serde(skip)]
        keyframe_samples: Vec<usize>,
        /// The cursor that calculates the gradient at the specific sample index.
        #[serde(skip)]
        float_cursor: FloatAutomationCursor,
    },
    /// Integer keyframe track, used for discrete values.
    /// The value changes instantly at the keyframe tick without interpolation.
    Int {
        /// The vector of keyframes, sorted by ticks.
        keyframes: Vec<Keyframe<i32>>,
        /// Cached sample indices of the keyframes, sorted in the order of the keyframe index.
        #[serde(skip)]
        keyframe_samples: Vec<usize>,
        /// The cursor that keeps track of the current position in the automation track for processing.
        #[serde(skip)]
        automation_cursor: ConstantAutomationCursor,
    },
    /// Boolean keyframe track.
    /// The value changes instantly at the keyframe tick without interpolation.
    Bool {
        /// The vector of keyframes, sorted by ticks.
        keyframes: Vec<Keyframe<bool>>,
        /// Cached sample indices of the keyframes, sorted in the order of the keyframe index.
        #[serde(skip)]
        keyframe_samples: Vec<usize>,
        /// The cursor that keeps track of the current position in the automation track for processing.
        #[serde(skip)]
        automation_cursor: ConstantAutomationCursor,
    },
}

impl AutomationTrack {
    // --- VALUE SIZE ---

    pub fn size_of_value(&self) -> usize {
        match self {
            AutomationTrack::Float { .. } => std::mem::size_of::<f32>(),
            AutomationTrack::Int { .. } => std::mem::size_of::<i32>(),
            AutomationTrack::Bool { .. } => std::mem::size_of::<bool>(),
        }
    }

    // --- CALCULATION ---

    pub fn prepare(&mut self, tempo_map: &TempoMap, playback_ctx: &PlaybackContext) {
        let sample_rate = playback_ctx.sample_rate;

        match self {
            AutomationTrack::Float {
                keyframes,
                keyframe_samples,
                float_cursor,
            } => {
                float_cursor.clear_cache();

                // Sort the keyframes by its ticks to ensure they are in the correct order for processing
                keyframes.sort_by_key(|k| k.ticks.0);

                // Calculate the sample indices of the keyframes based on the tempo map and sample rate
                keyframe_samples.clear();
                keyframe_samples.reserve(keyframes.len());

                for keyframe in keyframes.iter() {
                    let sample = tempo_map.ticks_to_samples(keyframe.ticks, sample_rate);
                    keyframe_samples.push(sample);
                }
            }
            AutomationTrack::Int {
                keyframes,
                keyframe_samples,
                automation_cursor,
            } => {
                // Clear the cursor cache
                automation_cursor.clear_cache();
                *keyframe_samples = keyframes
                    .iter()
                    .map(|keyframe| tempo_map.ticks_to_samples(keyframe.ticks, sample_rate))
                    .collect();
            }
            AutomationTrack::Bool {
                keyframes,
                keyframe_samples,
                automation_cursor,
            } => {
                // Clear the cursor cache
                automation_cursor.clear_cache();
                *keyframe_samples = keyframes
                    .iter()
                    .map(|keyframe| tempo_map.ticks_to_samples(keyframe.ticks, sample_rate))
                    .collect();
            }
        }
    }

    pub fn process(&mut self, buffer: &mut [u8], playhead: usize, playback_ctx: &PlaybackContext) {
        let buffer_end = playhead + playback_ctx.buffer_size;

        match self {
            AutomationTrack::Float {
                keyframes,
                keyframe_samples,
                float_cursor,
            } => {
                for (sample, chunk) in (playhead..buffer_end).zip(buffer.chunks_exact_mut(4)) {
                    let value =
                        float_cursor.get_interpolated_value(keyframes, keyframe_samples, sample);
                    chunk.copy_from_slice(&value.to_ne_bytes());
                }
            }
            AutomationTrack::Int {
                keyframes,
                keyframe_samples,
                automation_cursor,
            } => {
                for (sample, chunk) in (playhead..buffer_end).zip(buffer.chunks_exact_mut(4)) {
                    let value = automation_cursor.get_constant_keyframe_value::<i32>(
                        keyframes,
                        keyframe_samples,
                        sample,
                    );
                    chunk.copy_from_slice(&value.to_ne_bytes());
                }
            }
            AutomationTrack::Bool {
                keyframes,
                keyframe_samples,
                automation_cursor,
            } => {
                for (sample, chunk) in (playhead..buffer_end).zip(buffer.iter_mut()) {
                    let value = automation_cursor.get_constant_keyframe_value::<bool>(
                        keyframes,
                        keyframe_samples,
                        sample,
                    );
                    *chunk = value as u8;
                }
            }
        }
    }
}
