use crate::{
    data_types::Ticks,
    node::builtin::{AutomationTrack, CurveType},
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Keyframe<T> {
    pub tick: Ticks,
    pub curve: CurveType,
    pub value: T,
}

impl<T> Keyframe<T> {
    pub fn new(tick: Ticks, curve: CurveType, value: T) -> Self {
        Self { tick, curve, value }
    }
}

pub struct NormalizedKeyframe {
    pub index: usize,
    pub tick: Ticks,
    pub curve: CurveType,
    pub value: f32,
}

impl NormalizedKeyframe {
    pub fn new(index: usize, tick: Ticks, curve: CurveType, value: f32) -> Self {
        Self {
            index,
            tick,
            curve,
            value,
        }
    }
}

pub trait AutomationTarget: Sized {
    fn keyframes(track: &AutomationTrack) -> Option<&[Keyframe<Self>]>;

    fn keyframes_mut(track: &mut AutomationTrack) -> Option<&mut Vec<Keyframe<Self>>>;
}

impl AutomationTarget for f32 {
    fn keyframes(track: &AutomationTrack) -> Option<&[Keyframe<Self>]> {
        if let AutomationTrack::Float { keyframes, .. } = track {
            Some(keyframes)
        } else {
            None
        }
    }

    fn keyframes_mut(track: &mut AutomationTrack) -> Option<&mut Vec<Keyframe<Self>>> {
        if let AutomationTrack::Float { keyframes, .. } = track {
            Some(keyframes)
        } else {
            None
        }
    }
}

impl AutomationTarget for i32 {
    fn keyframes(track: &AutomationTrack) -> Option<&[Keyframe<Self>]> {
        if let AutomationTrack::Int { keyframes, .. } = track {
            Some(keyframes)
        } else {
            None
        }
    }

    fn keyframes_mut(track: &mut AutomationTrack) -> Option<&mut Vec<Keyframe<Self>>> {
        if let AutomationTrack::Int { keyframes, .. } = track {
            Some(keyframes)
        } else {
            None
        }
    }
}

impl AutomationTarget for bool {
    fn keyframes(track: &AutomationTrack) -> Option<&[Keyframe<Self>]> {
        if let AutomationTrack::Bool { keyframes, .. } = track {
            Some(keyframes)
        } else {
            None
        }
    }

    fn keyframes_mut(track: &mut AutomationTrack) -> Option<&mut Vec<Keyframe<Self>>> {
        if let AutomationTrack::Bool { keyframes, .. } = track {
            Some(keyframes)
        } else {
            None
        }
    }
}
