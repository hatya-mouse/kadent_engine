mod process;
mod render_worker;

pub(super) use render_worker::TrackSyncState;

use crate::{
    MAX_CHANNELS,
    data_types::{PlaybackContext, Ticks},
    graph::{Graph, error::GraphError},
    mixer::TempoMap,
    track::{
        RegionID, Track,
        audio_track::{AudioTrack, track_impl::render_worker::spawn_render_worker},
    },
};
use ringbuf::traits::{Consumer, Split};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

impl Track for AudioTrack {
    // --- CLONING ---

    fn clone_box(&self) -> Box<dyn Track> {
        Box::new(self.clone())
    }

    // --- GRAPH GETTING ---

    fn get_graph(&self) -> &Graph {
        &self.graph
    }

    fn get_graph_mut(&mut self) -> &mut Graph {
        &mut self.graph
    }

    // --- GRAPH UPDATING ---

    fn set_graph(&mut self, graph: Graph) {
        self.graph = graph;
    }

    // --- REGION MODIFICATION ---

    fn move_region(&mut self, region_id: &RegionID, new_start: Ticks) {
        if let Some(region) = self.regions.get_mut(region_id) {
            region.start = new_start;
        }
    }

    fn set_region_duration(&mut self, region_id: &RegionID, new_duration: Ticks) {
        if let Some(region) = self.regions.get_mut(region_id) {
            region.duration = new_duration;
        }
    }

    fn remove_region(&mut self, region_id: &RegionID) {
        self.regions.remove(region_id);
    }

    // --- SEEKING ---

    fn seek(&mut self, playhead: usize, _playback_ctx: &PlaybackContext) {
        if let Some(sync_state) = &self.sync_state {
            sync_state.request_seek(playhead);
        }
    }

    // --- TRACK PROCESSING ---

    fn prepare(
        &mut self,
        tempo_map: &TempoMap,
        playback_ctx: &PlaybackContext,
    ) -> Result<(), GraphError> {
        // Prepare the regions
        for region in self.regions.values_mut() {
            region.prepare(tempo_map);
        }

        // Stop the old render worker by setting the is_running flag to false
        if let Some(is_worker_running) = &self.is_worker_running {
            is_worker_running.store(false, Ordering::SeqCst);
        }

        // Create a new ring buffer and is_running flag for the new render worker
        let ringbuf_size = playback_ctx.buffer_size * MAX_CHANNELS * 2;
        let (prod, cons) = ringbuf::HeapRb::<f32>::new(ringbuf_size).split();
        self.ringbuf_cons = Some(cons);

        let is_worker_running = Arc::new(AtomicBool::new(true));
        is_worker_running.store(true, Ordering::SeqCst);
        self.is_worker_running = Some(is_worker_running.clone());

        // Create a sync state to share the playback position between threads
        let sync_state = TrackSyncState::new();
        self.sync_state = Some(sync_state.clone());

        // Spawn a new render worker thread
        spawn_render_worker(
            prod,
            self.regions.values().cloned().collect(),
            tempo_map.clone(),
            playback_ctx.clone(),
            is_worker_running,
            sync_state,
            0,
        );

        // Initialize the local buffers
        self.init_local_buffers(playback_ctx);
        // Then prepare the graph
        self.graph.prepare(tempo_map, playback_ctx)
    }

    fn process_to_local_buffer(
        &mut self,
        is_playing: bool,
        playhead: usize,
        _tempo_map: &TempoMap,
        playback_ctx: &PlaybackContext,
    ) {
        if is_playing {
            let buffer_len = MAX_CHANNELS * playback_ctx.buffer_size;
            if let Some(ringbuf_cons) = &mut self.ringbuf_cons {
                // Pop the rendered audio from the ring buffer into the local buffer
                let popped = ringbuf_cons.pop_slice(&mut self.graph_input_buffer[..buffer_len]);

                // Fill the rest of the local buffer with zeros if the ring buffer didn't have enough data
                if popped < buffer_len {
                    self.graph_input_buffer[popped..buffer_len].fill(0.0);
                }
            } else {
                // If the ring buffer consumer is not available, fill the local buffer with zeros
                self.graph_input_buffer.fill(0.0);
            }
        } else {
            // Mixer::process adds local_buffer into the output every callback regardless of
            // is_playing, so it must be cleared here to prevent the previous buffer from being played repeatedly
            self.graph_input_buffer.fill(0.0);
        }

        let input_ptr = self.graph_input_buffer.as_ptr() as *const u8;
        let output_ptr = self.local_buffer.as_mut_ptr() as *mut u8;

        self.graph
            .process(&[input_ptr], &[output_ptr], playhead, playback_ctx);
    }

    fn get_local_buffer(&self) -> &[f32] {
        &self.local_buffer
    }

    // --- ANY CASTING ---

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
