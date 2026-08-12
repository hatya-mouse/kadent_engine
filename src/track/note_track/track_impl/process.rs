use crate::{
    data_types::{Event, EventSlot, PlaybackContext, Ticks},
    mixer::TempoMap,
    track::note_track::{Note, NoteTrack, ProcessedNote, VoiceEvent, voice_event::VoiceEventKind},
};
use std::cmp::Reverse;

impl NoteTrack {
    // --- PREPARATION ---

    /// Extracts all notes from the regions and puts them into a HashMap with the key as (RegionID, NoteID) and the value as Note.
    pub(super) fn pre_process_notes(&mut self) {
        let mut notes: Vec<Note> = Vec::new();

        // Convert the local start Ticks to global Ticks by adding the start of the region
        for (_, region) in self.regions.iter() {
            for (_, note) in region.notes.iter() {
                // If the start of the note is after the end of the region
                // ...or if the end of the note is before the start of the region, skip it
                let note_end = note.start + note.duration;
                if note.start > region.duration || note_end < Ticks(0) {
                    continue;
                }

                // If the start of the note is before the start of the region, clamp it
                let clamped_start = note.start.max(Ticks(0));
                let absolute_start = clamped_start + region.start;
                // If the end of the note is after the end of the region, clamp it
                let clamped_duration = note_end.min(region.duration) - clamped_start;

                notes.push(Note {
                    start: absolute_start,
                    duration: clamped_duration,
                    pitch: note.pitch,
                    velocity: note.velocity,
                    modifiers: note.modifiers.clone(),
                });
            }
        }

        // Apply the modifiers to the notes
        let modified_notes = self.apply_modifiers(notes);

        // Convert the modified notes into ProcessedNote
        let mut processed_notes: Vec<ProcessedNote> = modified_notes
            .into_iter()
            .enumerate()
            .map(|(id, note)| ProcessedNote {
                id,
                start: note.start,
                duration: note.duration,
                pitch: note.pitch,
                velocity: note.velocity,
            })
            .collect();

        // Sort the notes by their start time and store them in the processed_notes field
        processed_notes.sort_by_key(|note| note.start);
        self.processed_notes = processed_notes;
    }

    /// Applies modifiers for notes.
    fn apply_modifiers(&mut self, notes: Vec<Note>) -> Vec<Note> {
        let mut processing_notes = notes;

        for (modifier_id, modifier) in self.modifiers.iter_mut() {
            // Partition the notes into those that have the modifier and those that don't
            let (target_notes, remaining_notes) = processing_notes
                .into_iter()
                .partition(|note| note.modifiers.contains(modifier_id));
            // Apply the modifier to the target notes
            let modified_notes = modifier.process(target_notes);

            // Merge the modified notes back into the processing notes for the next iteration
            processing_notes = remaining_notes.into_iter().chain(modified_notes).collect();
        }

        processing_notes
    }

    // --- PROCESS ---

    /// Retrieves the notes from the regions and converts them to events.
    pub(super) fn create_events_from_notes(
        &mut self,
        playhead: usize,
        tempo_map: &TempoMap,
        playback_ctx: &PlaybackContext,
    ) {
        let buffer_end = playhead + playback_ctx.buffer_size;

        for note in self.processed_notes.iter() {
            // Use samples for comparison to avoid asymmetric rounding
            let absolute_start_sample = tempo_map.ticks_to_samples(note.start);

            if absolute_start_sample < playhead {
                continue;
            } else if absolute_start_sample >= buffer_end {
                break;
            }

            let absolute_end_sample = tempo_map.ticks_to_samples(note.start + note.duration);

            // Add the note start and end event to the events
            self.voice_events.push(Reverse(VoiceEvent::new(
                absolute_start_sample,
                VoiceEventKind::NoteOn {
                    pitch: note.pitch,
                    velocity: note.velocity,
                },
            )));
            self.voice_events.push(Reverse(VoiceEvent::new(
                absolute_end_sample,
                VoiceEventKind::NoteOff { pitch: note.pitch },
            )));
        }
    }

    /// Converts voice events to events, and stores it to the local `event_buffer`.
    pub(super) fn consume_events_at_sample(&mut self, sample: usize) {
        let mut event_slot = EventSlot::default();

        // Process the delayed voice events first, if any
        while let Some(event) = self.delayed_voice_events.pop_front() {
            self.process_voice_event(event, &mut event_slot);
        }

        // Consume event and create events
        // Voice events must be sorted by sample time, so we can just peek at the first event and check if it is at the current sample
        while let Some(Reverse(event)) = self.voice_events.peek().cloned() {
            if event.sample_time > sample {
                // If the event is AFTER the current sample, break the loop
                break;
            } else if event.sample_time < sample {
                // If the event is BEFORE the current sample, consume it and continue the loop
                self.voice_events.pop();
                continue;
            }

            // Consume the event
            self.voice_events.pop();
            // Then process the voice event
            self.process_voice_event(event, &mut event_slot);
        }

        self.event_buffer.push(event_slot);
    }

    fn process_voice_event(&mut self, voice_event: VoiceEvent, event_slot: &mut EventSlot) {
        match &voice_event.kind {
            VoiceEventKind::NoteOn { pitch, velocity } => {
                println!(
                    "Processing NoteOn event: pitch={}, velocity={}",
                    pitch, velocity
                );
                if event_slot.is_full() {
                    self.delayed_voice_events.push_back(voice_event);
                } else {
                    event_slot.add_event(Event::new(1, *pitch, *velocity));
                    println!(
                        "Added NoteOn event to event_slot: pitch={}, velocity={}",
                        pitch, velocity
                    );
                }
            }
            VoiceEventKind::NoteOff { pitch } => {
                if event_slot.is_full() {
                    self.delayed_voice_events.push_back(voice_event);
                } else {
                    event_slot.add_event(Event::new(0, *pitch, 0.0));
                }
            }
        }
    }
}
