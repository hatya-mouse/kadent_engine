use crate::{
    data_types::{PlaybackContext, Ticks},
    graph::automation::{constant::ConstantAutomationCursor, float::FloatAutomationCursor},
    timing::TempoMap,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Keyframe<T> {
    pub ticks: Ticks,
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
        /// Cached gradient values before the specific sample for interpolation, sorted in the order of the sample index.
        ///
        /// The first value of the tuple is the **end** sample index of the range where corresponding gradient is applied,
        /// and the second value is the gradient value.
        ///
        /// ## Example
        /// If the keyframes are at ticks 100, 200, and 300 like below:
        ///
        /// ```
        /// |                     *                     *                     *
        /// 0                    100                   200                   300
        /// ```
        ///
        /// The gradient values will be stored in the vector like below:
        ///
        /// ```
        /// |<-- gradient value-->|<-- gradient value-->|<-- gradient value-->|
        ///                 end: 100              end: 200              end: 300
        /// ```
        ///
        /// This looks like:
        ///
        /// ```
        /// [(100, gradient_value_1), (200, gradient_value_2), (300, gradient_value_3)]
        /// ```
        #[serde(skip)]
        gradient_vals: Vec<(usize, f32)>,
        /// Cached sample indices of the keyframes, sorted in the order of the keyframe index.
        #[serde(skip)]
        keyframe_samples: Vec<usize>,
        /// The cursor that gets the constant value at the specific sample index.
        #[serde(skip)]
        const_cursor: ConstantAutomationCursor,
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

    pub fn prepare(&mut self, tempo_map: &TempoMap) {
        match self {
            AutomationTrack::Float {
                keyframes,
                gradient_vals,
                keyframe_samples,
                const_cursor,
                float_cursor,
            } => {
                // Calculate the keyframe samples
                keyframe_samples.clear();
                for keyframe in keyframes.iter() {
                    let sample = tempo_map.ticks_to_samples(keyframe.ticks);
                    keyframe_samples.push(sample);
                }

                // Clear the cursor cache
                const_cursor.clear_cache();
                float_cursor.clear_cache();

                gradient_vals.clear();
                if keyframes.is_empty() {
                    return;
                }

                // Reserve space for gradient values to avoid reallocations
                gradient_vals.reserve((keyframes.len() - 1).max(1));

                // Add zero gradient before the first keyframe
                let first_keyframe_samples = tempo_map.ticks_to_samples(keyframes[0].ticks);
                gradient_vals.push((first_keyframe_samples, 0.0));

                // If there are less than 2 keyframes, we cannot calculate gradients between them
                if keyframes.len() < 2 {
                    return;
                }

                // Calculate the gradient values between keyframes for quicker interpolation
                for i in 0..keyframes.len() - 1 {
                    let keyframe_1 = &keyframes[i];
                    let keyframe_2 = &keyframes[i + 1];

                    let delta_value = keyframe_2.value - keyframe_1.value;
                    let end_sample = tempo_map.ticks_to_samples(keyframe_2.ticks);
                    let delta_samples = end_sample - tempo_map.ticks_to_samples(keyframe_1.ticks);
                    let gradient_value = if delta_samples > 0 {
                        delta_value / delta_samples as f32
                    } else {
                        0.0
                    };

                    gradient_vals.push((end_sample, gradient_value));
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
                    .map(|keyframe| tempo_map.ticks_to_samples(keyframe.ticks))
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
                    .map(|keyframe| tempo_map.ticks_to_samples(keyframe.ticks))
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
                gradient_vals,
                const_cursor,
                float_cursor,
            } => {
                // First, get the constant value at the playhead position
                // Note that this is not the actual value at the playhead
                // |                 *   |<-- playhead
                // 0     const_value ^   ^ current_value
                let const_value =
                    const_cursor.get_constant_keyframe_value(keyframes, keyframe_samples, playhead);
                let keyframe_sample = const_cursor
                    .current_index
                    .map(|index| keyframe_samples[index])
                    .unwrap_or_default();
                let elapsed = playhead.saturating_sub(keyframe_sample);
                // Calculate the start value at the playhead position by adding the gradient value to the constant value
                let initial_gradient = float_cursor.get_gradient_at(gradient_vals, playhead);
                let mut current_value = const_value + elapsed as f32 * initial_gradient;

                for (sample, chunk) in (playhead..buffer_end).zip(buffer.chunks_exact_mut(4)) {
                    // Add the gradient value to the current value for each sample
                    let gradient = float_cursor.get_gradient_at(gradient_vals, sample);
                    chunk.copy_from_slice(&current_value.to_ne_bytes());
                    current_value += gradient;
                }
            }
            AutomationTrack::Int {
                keyframes,
                keyframe_samples,
                automation_cursor,
            } => {
                for (sample, chunk) in (playhead..buffer_end).zip(buffer.chunks_exact_mut(4)) {
                    let value = automation_cursor.get_constant_keyframe_value(
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
                    let value = automation_cursor.get_constant_keyframe_value(
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
