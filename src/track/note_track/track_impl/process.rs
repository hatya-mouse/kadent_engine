use crate::{
    data_types::Ticks,
    mixer::TempoMap,
    track::note_track::{
        Note, NoteTrack, ProcessedNote, VoiceEvent, VoiceSource,
        voice_event::{VoiceEventID, VoiceEventKind},
    },
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
                println!(
                    "Note Start: {}, Note Duration: {}",
                    note.start, note.duration
                );
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
    pub(super) fn create_events_from_notes(&mut self, playhead: usize, tempo_map: &TempoMap) {
        let buffer_end = playhead + self.audio_ctx.buffer_size;
        let playhead_ticks = tempo_map.samples_to_ticks(playhead);
        let buffer_end_ticks = tempo_map.samples_to_ticks(buffer_end);

        // Calculate the start sample of the region
        for note in self.processed_notes.iter() {
            // Skip the note if it is not in the currently processing buffer
            // Assume that processed_notes is sorted by start time, so we can break the loop if the note is after the buffer end
            if note.start < playhead_ticks {
                continue;
            } else if note.start >= buffer_end_ticks {
                break;
            }

            // Convert the start and end beats to samples
            let absolute_start_sample = tempo_map.ticks_to_samples(note.start);
            let absolute_end_sample = tempo_map.ticks_to_samples(note.start + note.duration);

            // Add the note start and end event to the events
            let voice_id = VoiceEventID::SequencedNote { id: note.id };
            self.voice_events.push(Reverse(VoiceEvent::new(
                absolute_start_sample,
                VoiceEventKind::NoteOn {
                    pitch: note.pitch,
                    velocity: note.velocity,
                },
                voice_id,
            )));
            self.voice_events.push(Reverse(VoiceEvent::new(
                absolute_end_sample,
                VoiceEventKind::NoteOff,
                voice_id,
            )));
        }
    }

    /// Updates the ages for each MIDI voices in `active_voices`.
    fn increment_midi_ages(&mut self) {
        let seconds_per_sample = 1f32 / self.audio_ctx.sample_rate as f32;
        self.active_voices
            .iter_mut()
            .zip(self.voice_sources.iter())
            .for_each(|(voice, source)| {
                if voice.is_active && matches!(source, Some(VoiceSource::RealtimeMidi)) {
                    voice.age += seconds_per_sample;
                }
            });
    }

    /// Updates the ages for each voices generated from sequenced `Note` in `active_voices`.
    fn increment_sequenced_ages(&mut self) {
        let seconds_per_sample = 1f32 / self.audio_ctx.sample_rate as f32;
        self.active_voices
            .iter_mut()
            .zip(self.voice_sources.iter())
            .for_each(|(voice, source)| {
                if voice.is_active && matches!(source, Some(VoiceSource::SequencedNote)) {
                    voice.age += seconds_per_sample;
                }
            });
    }

    /// Consumes the events at the current sample and updates the active voices.
    pub(super) fn consume_events_at_sample(&mut self, is_playing: bool, sample: usize) {
        // Increment ages for each active voices
        self.increment_midi_ages();
        if is_playing {
            self.increment_sequenced_ages();
        }

        // Consume event and create events
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

            match &event.kind {
                VoiceEventKind::NoteOn { pitch, velocity } => {
                    let new_index = self.find_or_steal_voice();
                    if let Some(voice) = self.active_voices.get_mut(new_index) {
                        voice.pitch = *pitch;
                        voice.velocity = *velocity;
                        voice.age = 0.0;
                        voice.is_active = true;
                    }
                    self.event_id_to_index.insert(event.id, new_index);

                    // Set the source of the voice based on the event ID
                    match event.id {
                        VoiceEventID::SequencedNote { .. } => {
                            self.voice_sources[new_index] = Some(VoiceSource::SequencedNote);
                        }
                        VoiceEventID::RealtimeMidi { .. } => {
                            self.voice_sources[new_index] = Some(VoiceSource::RealtimeMidi);
                        }
                    }
                }
                VoiceEventKind::NoteOff => {
                    if let Some(index) = self.event_id_to_index.remove(&event.id) {
                        self.free_voice(&index);
                        if let Some(voice) = self.active_voices.get_mut(index) {
                            voice.age = 0.0;
                            voice.is_active = false;
                        }

                        // Set the source of the voice to None
                        self.voice_sources[index] = None;
                    }
                }
            }
        }
    }
}
