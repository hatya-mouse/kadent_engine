mod note;
mod note_modifier;
mod note_region;
mod track_impl;
mod voice_event;

pub use note::{Note, NoteID};
pub use note_modifier::{NoteModifier, NoteModifierID};
pub use note_region::NoteRegion;

use crate::{
    data_types::{AudioContext, MidiEvent, Voice},
    graph::Graph,
    node::builtin::{AudioOutputNode, NoteInputNode},
    track::RegionID,
};
use std::collections::{BinaryHeap, HashMap, VecDeque};
use voice_event::VoiceEvent;

#[derive(Default, Clone)]
pub struct NoteTrack {
    // --- GRAPH ---
    graph: Graph,

    // --- NOTE DATA ---
    regions: HashMap<RegionID, NoteRegion>,

    // --- MODIFIERS ---
    modifiers: HashMap<NoteModifierID, Box<dyn NoteModifier>>,

    // --- EVENT -> VOICE PROCESSING ---
    /// Voice Events such as NoteOn and NoteOff.
    voice_events: BinaryHeap<VoiceEvent>,
    /// Active voices in the current frame.
    active_voices: Vec<Option<Voice>>,
    /// Indices in `active_voices` where the corresponding slots are vacant and available for new voice.
    free_voices: VecDeque<usize>,

    // --- LOCAL OUTPUT BUFFER ---
    local_buffer: Vec<f32>,

    // --- AUDIO CONTEXT ---
    audio_ctx: AudioContext,

    // --- MISC ---
    next_region_id: u64,
}

impl NoteTrack {
    pub fn new(audio_ctx: AudioContext) -> Self {
        // Create a graph with the input and output nodes
        let input_node = NoteInputNode::default();
        let output_node = AudioOutputNode::default();
        let graph = Graph::new(
            Box::new(input_node),
            Box::new(output_node),
            audio_ctx.clone(),
        );

        Self {
            graph,
            audio_ctx,
            ..Default::default()
        }
    }

    // --- REGION GETTING ---

    pub fn get_region(&self, id: &RegionID) -> Option<&NoteRegion> {
        self.regions.get(id)
    }

    pub fn get_region_mut(&mut self, id: &RegionID) -> Option<&mut NoteRegion> {
        self.regions.get_mut(id)
    }

    pub fn get_all_regions(&self) -> &HashMap<RegionID, NoteRegion> {
        &self.regions
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

    pub fn add_region(&mut self, region: NoteRegion) -> RegionID {
        let id = self.generate_region_id();
        self.regions.insert(id, region);
        id
    }

    pub fn set_regions(&mut self, regions: HashMap<RegionID, NoteRegion>) {
        self.regions = regions;
    }

    // --- REALTIME MIDI ---

    /// Receives live MIDI events and updates the voice state.
    /// Must be called before process() so that changes take effect from sample 0 of the buffer.
    pub fn pass_midi(&mut self, events: &[MidiEvent]) {
        for event in events {
            match event {
                MidiEvent::NoteOn { pitch, velocity } => {
                    // Allocate from the shared pool, stealing the oldest sequenced voice if full
                    let voice_idx = self
                        .free_voices
                        .pop()
                        .or_else(|| self.active_voices.pop_front())
                        .unwrap_or(0);
                    self.live_voices.insert(*pitch, voice_idx);
                    if let Some(v) = self.last_voices.get_mut(voice_idx) {
                        *v = Voice::new(*pitch as f32, *velocity as f32 / 127.0, 0.0, true);
                    }
                }
                MidiEvent::NoteOff { pitch } => {
                    if let Some(voice_idx) = self.live_voices.remove(pitch) {
                        self.free_voices.push(voice_idx);
                        if let Some(v) = self.last_voices.get_mut(voice_idx) {
                            v.is_active = false;
                            v.age = 0.0;
                        }
                    }
                }
            }
        }
    }
}
