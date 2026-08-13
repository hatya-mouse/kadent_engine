mod audio_data;
mod audio_region;
mod resampler;
mod tempo_strech;
mod track_impl;

pub use audio_data::{AudioDataInfo, AudioSource};
pub use audio_region::AudioRegion;

use crate::{
    graph::Graph,
    node::builtin::{AudioInputNode, AudioOutputNode},
    track::{RegionID, audio_track::track_impl::TrackSyncState},
};
use std::{
    collections::HashMap,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

#[derive(Default)]
pub struct AudioTrack {
    // --- GRAPH ---
    graph: Graph,

    // --- RAW AUDIO DATA ---
    regions: HashMap<RegionID, AudioRegion>,
    /// The pre-processed audio data, ready to be processed by the Graph.
    graph_input_buffer: Vec<f32>,

    // --- RENDER WORKER THREAD ---
    /// The ring buffer to receive the rendered audio data from the render thread.
    ringbuf_cons: Option<ringbuf::HeapCons<f32>>,
    /// Whether the worker thread should be running.
    is_worker_running: Option<Arc<AtomicBool>>,
    /// A sync state to synchronize the playhead position with the render worker thread.
    sync_state: Option<TrackSyncState>,

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
            regions: HashMap::new(),
            graph_input_buffer: Vec::new(),
            ringbuf_cons: None,
            is_worker_running: None,
            sync_state: None,
            local_buffer: Vec::new(),
            next_region_id: 0,
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

impl Clone for AudioTrack {
    fn clone(&self) -> Self {
        Self {
            graph: self.graph.clone(),
            regions: self.regions.clone(),
            graph_input_buffer: self.graph_input_buffer.clone(),
            ringbuf_cons: None,
            is_worker_running: None,
            sync_state: None,
            local_buffer: self.local_buffer.clone(),
            next_region_id: self.next_region_id,
        }
    }
}

impl Drop for AudioTrack {
    fn drop(&mut self) {
        // If the worker thread is running, signal it to stop
        if let Some(is_running) = &self.is_worker_running {
            is_running.store(false, Ordering::SeqCst);
        }
    }
}
