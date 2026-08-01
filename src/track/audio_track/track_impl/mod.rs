mod process;

use crate::{
    data_types::{AudioContext, Ticks},
    graph::{Graph, error::GraphError},
    mixer::TempoMap,
    track::{
        RegionID, Track,
        audio_track::{AudioTrack, tempo_strech::tempo_strech},
    },
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

    // --- AUDIO CONTEXT UPDARING ---

    fn set_audio_ctx(&mut self, audio_ctx: &AudioContext) {
        self.audio_ctx = audio_ctx.clone();
        self.graph.set_audio_ctx(audio_ctx);
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

    fn seek(&mut self, _playhead: usize) {}

    // --- TRACK PROCESSING ---

    fn prepare(&mut self, start: usize, tempo_map: &TempoMap) -> Result<(), GraphError> {
        // Calculate the total sample number
        // Ceil to a multiple of the buffer size
        self.playback_start_samples = start;

        // Calculate the total number of frames to process
        let start_index = start * self.audio_ctx.channels;
        let end_ticks = self
            .regions
            .values()
            .map(|r| r.start + r.duration)
            .max()
            .unwrap_or(Ticks(0));
        let end_samples = tempo_map.ticks_to_samples(end_ticks);
        let duration = end_samples - start;
        let total_samples =
            duration.div_ceil(self.audio_ctx.buffer_size) * self.audio_ctx.buffer_size;

        // Initialize the processed vector with zeros
        self.pre_processed = vec![0.0; total_samples * self.audio_ctx.channels];

        // Resample the each regions and add them to the pre_processed buffer
        for region in self.regions.values() {
            let resampled = tempo_strech(
                region,
                self.audio_ctx.sample_rate,
                self.audio_ctx.channels,
                tempo_map,
            );

            // Calculate the start sample index
            let region_start_index =
                tempo_map.ticks_to_samples(region.start) * self.audio_ctx.channels;
            // Calculate the start index in the pre_processed buffer
            // If the start_index is smaller than the region_start_index, we need to start copying the from the beginning of the region
            //
            // 0      start_index                          Start of the region  End of the region
            // |      |<-- start_index_in_pre_processed -->[                    ]
            // |      |<-- pre_processed starts from here
            let start_index_in_pre_processed = region_start_index.saturating_sub(start_index);

            // Add the resampled samples
            // Calculate the start index to copy from the resampled buffer
            // to copy the data efficiently when the start_index is larger than the region_start_index
            //
            // 0         region_start_index start_index                       End of the region
            // |         [<-- copy_start -->|<-- Copy from here to the end -->]
            let available = self.pre_processed.len().saturating_sub(region_start_index);
            let copy_start = start_index.saturating_sub(region_start_index);
            let copy_end = resampled.len().min(available);
            for (i, sample) in resampled[copy_start..copy_end].iter().enumerate() {
                self.pre_processed[start_index_in_pre_processed + i] += sample;
            }
        }

        // Initialize the local buffers
        self.init_local_buffers();

        // Then prepare the graph
        self.graph.prepare()
    }

    fn process_to_local_buffer(
        &mut self,
        is_playing: bool,
        playhead: usize,
        _tempo_map: &TempoMap,
    ) {
        if is_playing {
            let buffer_start = (playhead - self.playback_start_samples) * self.audio_ctx.channels;
            let buffer_size = self.audio_ctx.buffer_size * self.audio_ctx.channels;
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
            self.graph
                .process(&[input_ptr], &[self.local_buffer.as_mut_ptr() as *mut u8]);
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
