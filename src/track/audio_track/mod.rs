mod audio_region;
mod resampler;
mod tempo_strech;
mod track_impl;

pub use audio_region::AudioRegion;

use crate::{
    graph::Graph,
    node::builtin::{AudioInputNode, AudioOutputNode},
    track::RegionID,
};
use std::collections::HashMap;

#[derive(Default, Clone)]
pub struct AudioTrack {
    // --- GRAPH ---
    graph: Graph,

    // --- RAW AUDIO DATA ---
    regions: HashMap<RegionID, AudioRegion>,
    /// Pre-processed audio data for the audio track.
    /// The data is stored in the form of a interleaved buffer with `MAX_CHANNELS` channels.
    /// This means that this buffer can be reinterpreted as an array of `Sample` type, which has `MAX_CHANNELS` channels.
    pre_processed: Vec<f32>,

    // --- LOCAL BUFFER ---
    local_buffer: Vec<f32>,

    // --- MISC ---
    next_region_id: u64,
}

impl AudioTrack {
    pub fn new() -> Self {
        // Create a graph with the input and output nodes
        let input_node = AudioInputNode::default();
        let output_node = AudioOutputNode::default();
        let graph = Graph::new(Box::new(input_node), Box::new(output_node));

        Self {
            graph,
            ..Default::default()
        }
    }

    // --- REGION GETTING ---

    pub fn get_region(&self, id: &RegionID) -> Option<&AudioRegion> {
        self.regions.get(id)
    }

    pub fn get_region_mut(&mut self, id: &RegionID) -> Option<&mut AudioRegion> {
        self.regions.get_mut(id)
    }

    pub fn get_all_regions(&self) -> &HashMap<RegionID, AudioRegion> {
        &self.regions
    }

    pub fn take_region(&mut self, id: &RegionID) -> Option<AudioRegion> {
        self.regions.remove(id)
    }

    // --- REGION ADDITION ---

    pub fn set_next_region_id(&mut self, next_id: u64) {
        self.next_region_id = next_id;
    }

    fn generate_region_id(&mut self) -> RegionID {
        let id = RegionID(self.next_region_id);
        self.next_region_id += 1;
        id
    }

    pub fn add_region(&mut self, region: AudioRegion) -> RegionID {
        let id = self.generate_region_id();
        self.regions.insert(id, region);
        id
    }

    pub fn set_regions(&mut self, regions: HashMap<RegionID, AudioRegion>) {
        self.regions = regions;
    }
}
