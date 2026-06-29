mod process;

use crate::{
    data_types::{AudioContext, Ticks, Voice},
    graph::{Graph, error::GraphError},
    mixer::TempoMap,
    track::{RegionID, Track, note_track::NoteTrack},
};

impl Track for NoteTrack {
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

    // --- AUDIO CONTEXT UPDARING ---

    fn set_audio_ctx(&mut self, audio_ctx: &AudioContext) {
        self.audio_ctx = audio_ctx.clone();
        self.graph.set_audio_ctx(audio_ctx);
    }

    // --- SEEKING ---

    fn seek(&mut self, _playhead: usize) {}

    // --- TRACK PROCESSING ---

    fn prepare(
        &mut self,
        _start: usize,
        _duration: usize,
        _tempo_map: &TempoMap,
    ) -> Result<(), GraphError> {
        // Pre-process the sequenced notes into processed notes
        self.pre_process_notes();
        // Initialize the local buffer
        self.init_local_buffer();
        // Clear the voice events and fill the active_voices vector with inactive voices
        self.voice_events.clear();
        self.active_voices = vec![Voice::inactive(); self.audio_ctx.max_voices];

        // Prepare the graph
        self.graph.prepare()
    }

    fn process_to_local_buffer(
        &mut self,
        _is_playing: bool,
        playhead: usize,
        tempo_map: &TempoMap,
    ) {
        let mut voice_buffer =
            Vec::with_capacity(self.audio_ctx.buffer_size * self.audio_ctx.max_voices);
        let buffer_end = playhead + self.audio_ctx.buffer_size;

        // Set the midi_playhead to the start index of the next buffer
        // because the next buffer will start processing from that point
        self.midi_playhead = buffer_end;

        // Create voice events from sequenced notes
        self.create_events_from_notes(playhead, tempo_map);

        for sample in playhead..buffer_end {
            // Convert voice events to voices
            // Update active voics for this sample
            self.consume_events_at_sample(sample);
            // Extend the voice buffer with the current active voices
            voice_buffer.extend(self.active_voices.clone());
        }

        println!("Active voices: {:#?}", self.active_voices);

        // Get a pointer to the voice buffer
        let input_ptr = voice_buffer.as_ptr() as *const u8;
        // Process the graph
        self.graph
            .process(&[input_ptr], &[self.local_buffer.as_mut_ptr() as *mut u8]);
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
