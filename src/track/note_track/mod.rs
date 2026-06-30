mod note;
mod note_modifier;
mod note_region;
mod processed_note;
mod track_impl;
mod voice_event;
mod voice_source;

pub use note::{Note, NoteID};
pub use note_modifier::{NoteModifier, NoteModifierID};
pub use note_region::NoteRegion;

use crate::{
    data_types::{AudioContext, MidiEvent, Voice},
    graph::Graph,
    node::builtin::{AudioOutputNode, NoteInputNode},
    track::{
        RegionID,
        note_track::{
            processed_note::ProcessedNote, voice_event::VoiceEventID, voice_source::VoiceSource,
        },
    },
};
use std::{
    cmp::Reverse,
    collections::{BinaryHeap, HashMap, VecDeque},
};
use voice_event::VoiceEvent;

#[derive(Default, Clone)]
pub struct NoteTrack {
    // --- GRAPH ---
    graph: Graph,

    // --- NOTE DATA ---
    /// The original note data, which is not affected by the modifiers.
    regions: HashMap<RegionID, NoteRegion>,
    /// The processed note data, which has been processed by the modifiers.
    /// This is sorted by the start time of the notes, and is used for generating voice events.
    processed_notes: Vec<ProcessedNote>,

    // --- MODIFIERS ---
    modifiers: HashMap<NoteModifierID, Box<dyn NoteModifier>>,

    // --- VOICE EVENTS ---
    /// Voice Events such as NoteOn and NoteOff.
    /// Used for generating actual `Voice`.
    voice_events: BinaryHeap<Reverse<VoiceEvent>>,

    // --- EVENT -> VOICE PROCESSING ---
    /// *Active* voices in the currently processing frame. The length must be as the same as `max_voices`.
    active_voices: Vec<Voice>,
    /// The sources of each voices with each corresponding to voices in `active_voices`. (MIDI or SequencedNote)
    voice_sources: Vec<Option<VoiceSource>>,
    /// Indices where the corresponding slots are vacant and available for new voice.
    /// Indices are of `active_voices`.
    ///
    /// It is recommended to call `pop_front` to get a free voice, and call `push_back` to register a free slot.
    free_voices: VecDeque<usize>,

    // --- ACTIVE VOICES MANAGEMENT ---
    /// Map from `VoiceEventID` to `active_voices`, used to get the corresponding voice when processing NoteOff.
    event_id_to_index: HashMap<VoiceEventID, usize>,

    // --- MIDI VOICE INSERTION ---
    /// Pending MIDI events to be processied in the next buffer.
    pending_midi_events: Vec<MidiEvent>,

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
        // If there is a free voice, return it
        if let Some(index) = self.free_voices.pop_front() {
            return index;
        }

        // If not, find the oldest active voice and return its index
        let stolen_index = self
            .active_voices
            .iter()
            .enumerate()
            .filter(|(_, v)| v.is_active)
            .max_by(|(_, a), (_, b)| a.age.partial_cmp(&b.age).unwrap())
            .map(|(i, _)| i)
            .unwrap_or(0);
        // Remove the old voice from the event_id_to_index map
        self.event_id_to_index.retain(|_, &mut v| v != stolen_index);

        stolen_index
    }

    /// Marks the given voice index free.
    fn free_voice(&mut self, free_index: &usize) {
        self.free_voices.push_back(*free_index);
    }

    // --- REALTIME MIDI ---

    /// Receives live MIDI events and updates the voice state.
    /// Must be called before process() so that changes take effect from sample 0 of the buffer.
    pub fn pass_midi(&mut self, events: &[MidiEvent]) {
        // Push a new voice event to the queue
        self.pending_midi_events.extend(events.to_vec());
    }
}
