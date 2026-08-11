mod constant;
mod float;
mod track;

use serde::{Deserialize, Serialize};
pub use track::{AutomationTrack, Keyframe};

use crate::{data_types::PlaybackContext, graph::node_id::NodeID, mixer::TempoMap};
use std::collections::HashMap;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct KeyframeManager {
    /// The keyframe tracks for each node and input index.
    pub keyframe_tracks: HashMap<(NodeID, usize), AutomationTrack>,
}

impl KeyframeManager {
    // --- PREPARE ---

    pub(super) fn prepare(&mut self, tempo_map: &TempoMap) {
        for track in self.keyframe_tracks.values_mut() {
            track.prepare(tempo_map);
        }
    }

    // --- PROCESS ---

    pub(super) fn process(
        &mut self,
        keyframe_buffers: &mut HashMap<(NodeID, usize), Vec<u8>>,
        playhead: usize,
        playback_ctx: &PlaybackContext,
    ) {
        for (key, track) in self.keyframe_tracks.iter_mut() {
            if let Some(buffer) = keyframe_buffers.get_mut(key) {
                track.process(buffer, playhead, playback_ctx);
            }
        }
    }
}
