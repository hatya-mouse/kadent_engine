mod track;

use crate::graph::{keyframe::track::KeyframeTrack, node_id::NodeID};
use std::collections::HashMap;

pub struct KeyframeManager {
    /// The keyframe tracks managed by this manager.
    output_buffers: HashMap<(NodeID, usize), KeyframeTrack>,
}
