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

pub trait AutomationTarget: Sized {
    fn keyframes_mut(track: &mut AutomationTrack) -> Option<&mut Vec<Keyframe<Self>>>;
}

impl AutomationTarget for f32 {
    fn keyframes_mut(track: &mut AutomationTrack) -> Option<&mut Vec<Keyframe<Self>>> {
        if let AutomationTrack::Float { keyframes, .. } = track {
            Some(keyframes)
        } else {
            None
        }
    }
}

impl AutomationTarget for i32 {
    fn keyframes_mut(track: &mut AutomationTrack) -> Option<&mut Vec<Keyframe<Self>>> {
        if let AutomationTrack::Int { keyframes, .. } = track {
            Some(keyframes)
        } else {
            None
        }
    }
}

impl AutomationTarget for bool {
    fn keyframes_mut(track: &mut AutomationTrack) -> Option<&mut Vec<Keyframe<Self>>> {
        if let AutomationTrack::Bool { keyframes, .. } = track {
            Some(keyframes)
        } else {
            None
        }
    }
}
