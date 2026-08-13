mod process;

use crate::{
    MAX_CHANNELS,
    data_types::{PlaybackContext, Ticks},
    graph::{Graph, error::GraphError},
    mixer::TempoMap,
    track::{RegionID, Track, audio_track::AudioTrack},
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

    fn seek(&mut self, _playhead: usize, _playback_ctx: &PlaybackContext) {}

    // --- TRACK PROCESSING ---

    fn prepare(
        &mut self,
        tempo_map: &TempoMap,
        playback_ctx: &PlaybackContext,
    ) -> Result<(), GraphError> {
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
            // Calculate the number of f32 values to process for the current buffer
            let buffer_start = playhead * MAX_CHANNELS;
            let buffer_size = playback_ctx.buffer_size * MAX_CHANNELS;
            let buffer_end = buffer_start + buffer_size;

            // Create a vector for input buffer
            let mut input_vec: Vec<f32>;

            let input_ptr = if buffer_end <= self.pre_processed.len() {
                // Get a pointer to the input buffer
                self.pre_processed[buffer_start..buffer_end].as_ptr() as *const u8
            } else {
                // If the audio data for the buffer is partially unavailable fill the rest with zero
                let available = self.pre_processed.len().saturating_sub(buffer_start);
                input_vec = vec![0f32; buffer_size];
                if available > 0 {
                    input_vec[..available].copy_from_slice(
                        &self.pre_processed[buffer_start..buffer_start + available],
                    );
                }
                input_vec.as_ptr() as *const u8
            };

            // Process the graph
            self.graph.process(
                &[input_ptr],
                &[self.local_buffer.as_mut_ptr() as *mut u8],
                playhead,
                playback_ctx,
            );
        } else {
            // Mixer::process adds local_buffer into the output every callback regardless of
            // is_playing, so it must be cleared here to prevent the previous buffer from being played repeatedly
            self.local_buffer.fill(0.0);
        }
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
