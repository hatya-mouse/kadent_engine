mod process;

use crate::{
    data_types::{AudioContext, Ticks},
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
        // Initialize the local buffer
        self.init_local_buffer();
        // Prepare the graph
        self.graph.prepare()
    }

    fn process_to_local_buffer(&mut self, is_playing: bool, playhead: usize) {
        let mut voice_buffer =
            Vec::with_capacity(self.audio_ctx.buffer_size * self.audio_ctx.max_voices);
        let buffer_end = playhead + self.audio_ctx.buffer_size;

        // Create voice events from MIDI input and sequenced notes

        for sample in playhead..buffer_end {
            // Convert voice events to voices
            // Update active voics for this sample
            self.consume_events_at_sample(sample);
        }

        // --------- OLD ----------

        // Convert the playhead beats to samples
        let buffer_end = playhead + self.audio_ctx.buffer_size;
        let max_voices = self.audio_ctx.max_voices;

        // Seek the event cursor to the current playhead position
        self.seek_event_cursor(playhead);

        for sample in playhead..buffer_end {
            // Calculate the local sample in the buffer chunk
            let local_sample = sample - playhead;
            // Calculate the index of the first voice for the
            // current sample in the voice buffer
            let first_voice_index = local_sample * max_voices;

            // Copy the voice data from the previous sample
            self.propagate_voices(local_sample, max_voices, first_voice_index);
            // Increment age for each live voices
            self.increment_live_ages(first_voice_index);

            // Process the sequenced voices when playing
            if is_playing {
                self.consume_events_at_sample(sample, first_voice_index);
            }

            // Ramp gain for active and releasing voices
            self.calculate_gains(first_voice_index);
        }

        // Copy the last voices
        let last = (self.audio_ctx.buffer_size - 1) * max_voices;
        self.last_voices
            .clone_from_slice(&self.voice_buffer[last..last + max_voices]);

        // Get a pointer to the voice buffer
        let input_ptr = self.voice_buffer.as_ptr() as *const u8;
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
