mod constant;
mod float;
mod track;

pub use track::{AutomationTrack, Keyframe};

use crate::graph::node_id::NodeID;
use std::collections::HashMap;

#[derive(Debug, Clone, Default)]
pub struct KeyframeManager {
    /// The keyframe tracks for each node and input index.
    keyframe_tracks: HashMap<(NodeID, usize), AutomationTrack>,
    /// Calculated keyframe values for the current buffer.
    keyframe_buffers: HashMap<(NodeID, usize), Vec<u8>>,
}

impl KeyframeManager {
    /// Allocates memory for the calculated keyframe data.
    pub fn allocate_input_sources(&mut self) {
        for (key, track) in &self.keyframe_tracks {
            let buffer = vec![0u8; track.size_of_value()];
            self.keyframe_buffers.insert(*key, buffer);
        }
    }
}
