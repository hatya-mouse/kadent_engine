use crate::{
    data_types::{PlaybackContext, Ticks},
    node::builtin::{
        Keyframe,
        automation_node::{
            AutomationTarget, NormalizedKeyframe, constant::ConstantAutomationCursor,
            float::FloatAutomationCursor,
        },
    },
    timing::TempoMap,
};
use serde::{Deserialize, Serialize};
use std::ops::{Range, RangeInclusive};

/// A track that stores keyframes for a specific node and input index.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AutomationTrack {
    /// Float keyframe track, used for continuous values.
    /// The value is interpolated between keyframes based on the gradient calculated during preparation.
    Float {
        /// The vector of keyframes, sorted by ticks.
        keyframes: Vec<Keyframe<f32>>,
        /// Inclusive range of the keyframe values, used for clamping the output values.
        range: RangeInclusive<f32>,
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
        /// Inclusive range of the keyframe values, used for clamping the output values.
        range: RangeInclusive<i32>,
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
    // --- INITIALIZATION ---

    pub fn new_float(range: RangeInclusive<f32>) -> Self {
        AutomationTrack::Float {
            keyframes: Vec::new(),
            range,
            keyframe_samples: Vec::new(),
            float_cursor: FloatAutomationCursor::default(),
        }
    }

    pub fn new_int(range: RangeInclusive<i32>) -> Self {
        AutomationTrack::Int {
            keyframes: Vec::new(),
            range,
            keyframe_samples: Vec::new(),
            automation_cursor: ConstantAutomationCursor::default(),
        }
    }

    pub fn new_bool() -> Self {
        AutomationTrack::Bool {
            keyframes: Vec::new(),
            keyframe_samples: Vec::new(),
            automation_cursor: ConstantAutomationCursor::default(),
        }
    }

    // --- ITERATOR ---

    /// Returns an vector over the float representation of the value in the given tick range and the last and the first keyframes outside the range.
    pub fn normalized_keyframes_around(&self, tick_range: Range<Ticks>) -> Vec<NormalizedKeyframe> {
        match self {
            AutomationTrack::Float {
                keyframes, range, ..
            } => {
                let min = *range.start();
                let range = (range.end() - range.start()).max(1e-6);
                if let Some((first_index, keyframes)) =
                    Self::keyframes_around_range(keyframes, tick_range)
                {
                    keyframes
                        .iter()
                        .enumerate()
                        .map(|(index, keyframe)| {
                            let normalized = (keyframe.value - min) / range;
                            NormalizedKeyframe::new(
                                first_index + index,
                                keyframe.tick,
                                keyframe.curve,
                                normalized,
                            )
                        })
                        .collect()
                } else {
                    Vec::new()
                }
            }
            AutomationTrack::Int {
                keyframes, range, ..
            } => {
                let min = *range.start();
                let range = ((range.end() - range.start()) as f32).max(1e-6);
                if let Some((first_index, keyframes)) =
                    Self::keyframes_around_range(keyframes, tick_range)
                {
                    keyframes
                        .iter()
                        .enumerate()
                        .map(|(index, keyframe)| {
                            let normalized = (keyframe.value - min) as f32 / range;
                            NormalizedKeyframe::new(
                                first_index + index,
                                keyframe.tick,
                                keyframe.curve,
                                normalized,
                            )
                        })
                        .collect()
                } else {
                    Vec::new()
                }
            }
            AutomationTrack::Bool { keyframes, .. } => {
                if let Some((first_index, keyframes)) =
                    Self::keyframes_around_range(keyframes, tick_range)
                {
                    keyframes
                        .iter()
                        .enumerate()
                        .map(|(index, keyframe)| {
                            let normalized = if keyframe.value { 1.0 } else { 0.0 };
                            NormalizedKeyframe::new(
                                first_index + index,
                                keyframe.tick,
                                keyframe.curve,
                                normalized,
                            )
                        })
                        .collect()
                } else {
                    Vec::new()
                }
            }
        }
    }

    /// Returns an slice to the keyframes in the given tick range and the last and the first keyframes outside the range.
    fn keyframes_around_range<T>(
        keyframes: &[Keyframe<T>],
        tick_range: Range<Ticks>,
    ) -> Option<(usize, &[Keyframe<T>])> {
        // Return if there are no keyframes
        if keyframes.is_empty() {
            return Some((0, keyframes));
        }

        let first_index = keyframes
            .partition_point(|k| k.tick < tick_range.start)
            .saturating_sub(1);
        let last_index = keyframes
            .partition_point(|k| k.tick < tick_range.end)
            .saturating_add(1)
            .min(keyframes.len());

        Some((first_index, &keyframes[first_index..last_index]))
    }

    /// Returns the index of the first keyframe that is greater than or equal to the given tick range.
    fn keyframe_partition_point(&self, tick_range: Ticks) -> usize {
        match self {
            AutomationTrack::Float { keyframes, .. } => {
                keyframes.partition_point(|keyframe| keyframe.tick < tick_range)
            }
            AutomationTrack::Int { keyframes, .. } => {
                keyframes.partition_point(|keyframe| keyframe.tick < tick_range)
            }
            AutomationTrack::Bool { keyframes, .. } => {
                keyframes.partition_point(|keyframe| keyframe.tick < tick_range)
            }
        }
    }

    // --- MODIFICATION ---

    /// Returns a mutable reference to the float keyframes vector.
    pub fn get_float_keyframes_mut(&mut self) -> Option<&mut Vec<Keyframe<f32>>> {
        f32::keyframes_mut(self)
    }

    /// Returns a mutable reference to the integer keyframes vector.
    pub fn get_int_keyframes_mut(&mut self) -> Option<&mut Vec<Keyframe<i32>>> {
        i32::keyframes_mut(self)
    }

    /// Returns a mutable reference to the boolean keyframes vector.
    pub fn get_bool_keyframes_mut(&mut self) -> Option<&mut Vec<Keyframe<bool>>> {
        bool::keyframes_mut(self)
    }

    /// Adds a new keyframe to the track, maintaining the sorted order by ticks.
    pub fn add_keyframe<T>(&mut self, keyframe: Keyframe<T>) -> usize
    where
        T: AutomationTarget,
    {
        let index = self.keyframe_partition_point(keyframe.tick);
        if let Some(keyframes) = T::keyframes_mut(self) {
            keyframes.insert(index, keyframe);
        }
        index
    }

    // --- CALCULATION ---

    pub fn prepare(&mut self, tempo_map: &TempoMap, playback_ctx: &PlaybackContext) {
        let sample_rate = playback_ctx.sample_rate;

        match self {
            AutomationTrack::Float {
                keyframes,
                range: _,
                keyframe_samples,
                float_cursor,
            } => {
                float_cursor.clear_cache();

                // Sort the keyframes by its ticks to ensure they are in the correct order for processing
                keyframes.sort_by_key(|k| k.tick.0);

                // Calculate the sample indices of the keyframes based on the tempo map and sample rate
                keyframe_samples.clear();
                keyframe_samples.reserve(keyframes.len());

                for keyframe in keyframes.iter() {
                    let sample = tempo_map.ticks_to_samples(keyframe.tick, sample_rate);
                    keyframe_samples.push(sample);
                }
            }
            AutomationTrack::Int {
                keyframes,
                range: _,
                keyframe_samples,
                automation_cursor,
            } => {
                // Clear the cursor cache
                automation_cursor.clear_cache();
                *keyframe_samples = keyframes
                    .iter()
                    .map(|keyframe| tempo_map.ticks_to_samples(keyframe.tick, sample_rate))
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
                    .map(|keyframe| tempo_map.ticks_to_samples(keyframe.tick, sample_rate))
                    .collect();
            }
        }
    }

    pub fn process(&mut self, buffer: &mut [u8], playhead: usize, playback_ctx: &PlaybackContext) {
        let buffer_end = playhead + playback_ctx.buffer_size;

        match self {
            AutomationTrack::Float {
                keyframes,
                range,
                keyframe_samples,
                float_cursor,
            } => {
                for (sample, chunk) in (playhead..buffer_end).zip(buffer.chunks_exact_mut(4)) {
                    let value = float_cursor
                        .get_interpolated_value(keyframes, keyframe_samples, sample)
                        .clamp(*range.start(), *range.end());
                    chunk.copy_from_slice(&value.to_ne_bytes());
                }
            }
            AutomationTrack::Int {
                keyframes,
                range,
                keyframe_samples,
                automation_cursor,
            } => {
                for (sample, chunk) in (playhead..buffer_end).zip(buffer.chunks_exact_mut(4)) {
                    let value = automation_cursor
                        .get_constant_keyframe_value::<i32>(keyframes, keyframe_samples, sample)
                        .clamp(*range.start(), *range.end());
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
