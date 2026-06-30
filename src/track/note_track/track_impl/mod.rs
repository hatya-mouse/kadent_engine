mod process;

use crate::{
    data_types::{HardwareConfig, ProjectConfig, Ticks, Voice},
    graph::{Graph, error::GraphError},
    mixer::TempoMap,
    track::{
        RegionID, Track,
        note_track::{NoteTrack, VoiceEvent},
    },
};
use std::cmp::Reverse;

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

    // --- PROJECT CONTEXT UPDARING ---

    fn set_config(&mut self, proj_config: &ProjectConfig, hardware_config: &HardwareConfig) {
        self.proj_config = proj_config.clone();
        self.hardware_config = hardware_config.clone();
        self.graph.set_config(proj_config, hardware_config);
    }

    // --- SEEKING ---

    fn seek(&mut self, _playhead: usize) {
        // Clear the voices and events
        let max_voices = self.hardware_config.max_voices as usize;
        self.voice_events.clear();
        self.active_voices = vec![Voice::default(); max_voices];
        self.voice_sources = vec![None; max_voices];
        self.free_voices = (0..max_voices).collect();
    }

    // --- TRACK PROCESSING ---

    fn prepare(
        &mut self,
        _start: usize,
        _duration: usize,
        _tempo_map: &TempoMap,
    ) -> Result<(), GraphError> {
        // Pre-process the sequenced notes into processed notes
        self.pre_process_notes();

        // Clear the voices and events
        let max_voices = self.hardware_config.max_voices as usize;
        self.voice_events.clear();
        self.active_voices = vec![Voice::default(); max_voices];
        self.voice_sources = vec![None; max_voices];
        self.free_voices = (0..max_voices).collect();

        // Initialize the local buffer
        self.local_buffer = vec![
            0.0;
            self.hardware_config.buffer_size as usize
                * self.proj_config.channels as usize
        ];

        // Prepare the graph
        self.graph.prepare()
    }

    fn process_to_local_buffer(&mut self, is_playing: bool, playhead: usize, tempo_map: &TempoMap) {
        let buffer_size = self.hardware_config.buffer_size as usize;
        let mut voice_buffer =
            Vec::with_capacity(buffer_size * self.hardware_config.max_voices as usize);
        let buffer_end = playhead + buffer_size;

        // Convert the pending MIDI notes to voice events and push them to the voice_events vector
        let converted_midi_events: Vec<Reverse<VoiceEvent>> = self
            .pending_midi_events
            .drain(..)
            .map(|midi_event| Reverse(VoiceEvent::from_midi_event(playhead, midi_event)))
            .collect();
        self.voice_events.extend(converted_midi_events);

        if is_playing {
            // Create voice events from sequenced notes
            self.create_events_from_notes(playhead, tempo_map);
        }

        for sample in playhead..buffer_end {
            // Convert voice events to voices
            // Update active voics for this sample
            self.consume_events_at_sample(is_playing, sample);
            // Extend the voice buffer with the current active voices
            voice_buffer.extend(self.active_voices.clone());
        }

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
