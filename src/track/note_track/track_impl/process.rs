use std::cmp::Reverse;

use crate::{
    mixer::TempoMap,
    track::note_track::{
        NoteTrack, VoiceEvent,
        voice_event::{VoiceEventID, VoiceEventKind},
    },
};

impl NoteTrack {
    // --- PREPARATION ---

    /// Retrieves the notes from the regions and converts them to events.
    pub(super) fn create_events_from_notes(&mut self, tempo_map: &TempoMap) {
        for (region_id, region) in self.regions.iter() {
            let region_end = region.start + region.duration;

            // Calculate the start sample of the region
            for (note_id, note) in region.notes.iter() {
                // let note_end = note.start + note.duration;

                // Calculate the start and end sample of the note in the entire track
                let absolute_note_start = region.start + note.start;
                let absolute_note_end = absolute_note_start + note.duration;

                // Skip the note if it is outside the region
                // Skip if absolute_note_start equals region_end to prevent NOTE OFF event
                // from occuring at the same time as the NOTE ON
                if absolute_note_start >= region_end || absolute_note_end < region.start {
                    continue;
                }

                // Clamp the start and the end beats by the region start and the end
                let clamped_note_start = absolute_note_start.max(region.start);
                let clamped_note_end = absolute_note_end.min(region_end);

                // Convert the start and end beats to samples
                let absolute_start_sample = tempo_map.ticks_to_samples(clamped_note_start);
                let absolute_end_sample = tempo_map.ticks_to_samples(clamped_note_end);

                // Add the note start and end event to the events
                let voice_id = VoiceEventID::SequencedNote {
                    region_id: *region_id,
                    note_id: *note_id,
                };
                self.voice_events.push(Reverse(VoiceEvent::new(
                    absolute_start_sample,
                    VoiceEventKind::NoteOn {
                        pitch: note.pitch,
                        velocity: note.velocity,
                    },
                    voice_id.clone(),
                )));
                self.voice_events.push(Reverse(VoiceEvent::new(
                    absolute_end_sample,
                    VoiceEventKind::NoteOff,
                    voice_id.clone(),
                )));
            }
        }
    }

    /// Initializes the local buffer based on the buffer size.
    pub(super) fn init_local_buffer(&mut self) {
        self.local_buffer = vec![0.0; self.audio_ctx.buffer_size * self.audio_ctx.channels];
    }

    // --- PROCESS ---

    /// Updates the ages for each active voices.
    fn increment_active_ages(&mut self) {
        let seconds_per_sample = 1f32 / self.audio_ctx.sample_rate as f32;
        for active_voice in self.active_voices.iter_mut() {
            active_voice.age += seconds_per_sample;
        }
    }

    /// Consumes the events at the current sample and updates the active voices.
    pub(super) fn consume_events_at_sample(&mut self, sample: usize) {
        // Increment ages for each active voices
        self.increment_active_ages();

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
                }
                VoiceEventKind::NoteOff => {
                    if let Some(index) = self.event_id_to_index.get(&event.id).copied() {
                        self.free_voice(&index);
                        if let Some(voice) = self.active_voices.get_mut(index) {
                            voice.age = 0.0;
                            voice.is_active = false;
                        }
                    }
                }
            }
        }
    }
}
