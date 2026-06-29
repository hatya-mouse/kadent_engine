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
    track::{RegionID, note_track::voice_event::VoiceEventID},
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

    // --- VOICE EVENTS ---
    /// Voice Events such as NoteOn and NoteOff.
    /// Used for generating actual `Voice`.
    voice_events: BinaryHeap<VoiceEvent>,

    // --- EVENT -> VOICE PROCESSING ---
    /// *Active* voices in the currently processing frame. The length must be as the same as `max_voices`.
    active_voices: Vec<Voice>,
    /// Indices where the corresponding slots are vacant and available for new voice.
    /// Indices are of `active_voices`.
    /// It is recommended to call `pop_front` to get a free voice, and call `push_back` to register a free slot.
    free_voices: VecDeque<usize>,
    /// Indices of currently *active* voices, sorted in a order where the voices have started.
    /// Indices are of `active_voices`.
    /// Call `pop_front` to get an oldest voice, and call `push_back` to register a new voice.
    old_voices: VecDeque<usize>,

    // --- ACTIVE VOICES MANAGEMENT ---
    /// Map from `VoiceEventID` to `active_voices`, used to get the corresponding voice when processing NoteOff.
    event_id_to_index: HashMap<VoiceEventID, usize>,

    // --- MIDI VOICE INSERTION ---
    /// The next available sample index for real-time MIDI event to be added.
    midi_playhead: usize,

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

    // --- VOICE STEALING ---

    /// Returns the vacant voice index, or returns the index of the oldest voice.
    /// This function registers the given voice index to `old_voices`.
    fn find_or_steal_voice(&mut self) -> usize {
        // TODO: NOT TO USE VECDEQUE FOR `old_voices` BECAUSE SOME VOICES MAY END BEFORE THE OLDER VOICE ENDS
        let new_index = self
            .free_voices
            .pop_front()
            .unwrap_or_else(|| self.old_voices.pop_front().unwrap_or_default());
        self.old_voices.push_back(new_index);
        new_index
    }

    /// Marks the given voice index free.
    fn free_voice(&mut self, free_index: &usize) {
        self.free_voices.retain(|index| index != free_index);
    }

    // --- REALTIME MIDI ---

    /// Receives live MIDI events and updates the voice state.
    /// Must be called before process() so that changes take effect from sample 0 of the buffer.
    pub fn pass_midi(&mut self, events: &[MidiEvent]) {
        for event in events {
            match event {
                MidiEvent::NoteOn { pitch, velocity } => {
                    // Allocate from the shared pool, stealing the oldest sequenced voice if full
                    let voice_idx = self.find_or_steal_voice();
                    self.voice_events.push(VoiceEvent::from_midi_event(
                        self.midi_playhead,
                        event.clone(),
                    ));
                }
                MidiEvent::NoteOff { pitch } => {}
            }
        }
    }
}
