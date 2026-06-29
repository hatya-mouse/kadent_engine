use crate::{
    data_types::Voice,
    mixer::TempoMap,
    track::note_track::{NoteTrack, VoiceEvent},
};
use std::ptr::copy_nonoverlapping;

const DECLICK_SAMPLES: usize = 512;

impl NoteTrack {
    // --- VOICE GETTING ---

    /// Returns the vacant voice index, or returns the index of the oldest voice.
    pub(super) fn find_or_steal_voice(&mut self) -> usize {
        let new_voice_index = self.free_voices.pop().unwrap_or_else(|| {
            // Drain stale entries from the front until we find one still active
            loop {
                let idx = self.active_voices.pop_front().unwrap_or(0);
                if self.active_voice_set.remove(&idx) {
                    break idx;
                }
            }
        });
        // Cancel any in-progress fade-out if this slot is being reused
        self.released_voice_set.remove(&new_voice_index);
        self.active_voices.push_back(new_voice_index);
        self.active_voice_set.insert(new_voice_index);
        new_voice_index
    }

    // --- PREPARATION ---

    /// Retrieves the notes from the regions and converts them to events.
    pub(super) fn retrieve_and_register_notes(&mut self, tempo_map: &TempoMap) {
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
                self.events.push(VoiceEvent::new(
                    (*region_id, *note_id),
                    absolute_start_sample,
                    note.pitch,
                    note.velocity,
                    true,
                ));
                self.events.push(VoiceEvent::new(
                    (*region_id, *note_id),
                    absolute_end_sample,
                    note.pitch,
                    note.velocity,
                    false,
                ));
            }
        }
    }

    /// Initializes the local buffer based on the buffer size.
    pub(super) fn init_local_buffer(&mut self) {
        self.local_buffer = vec![0.0; self.audio_ctx.buffer_size * self.audio_ctx.channels];
    }

    // --- PROCESS ---

    /// Seeks the event cursor to the current playhead position.
    pub(super) fn seek_event_cursor(&mut self, playhead: usize) {
        if self
            .events
            .get(self.event_cursor)
            .is_some_and(|e| e.sample_index > playhead)
            || (self.event_cursor > 0 && self.events[self.event_cursor - 1].sample_index > playhead)
        {
            self.event_cursor = self.events.partition_point(|e| e.sample_index < playhead);
        }
    }

    /// Propagates the voice data from the previous sample to the current sample.
    pub(super) fn propagate_voices(
        &mut self,
        local_sample: usize,
        max_voices: usize,
        first_voice_index: usize,
    ) {
        // If the current sample is the first sample in the buffer,
        // Copy from the last voices
        if local_sample == 0 && !self.last_voices.is_empty() {
            unsafe {
                copy_nonoverlapping(
                    self.last_voices.as_ptr(),
                    self.voice_buffer.as_mut_ptr(),
                    max_voices,
                );
            }
        }

        // If the current sample is not the first sample in the buffer,
        // copy the previous voices to the current index
        if local_sample > 0 {
            let previous = (local_sample - 1) * max_voices;
            unsafe {
                copy_nonoverlapping(
                    self.voice_buffer[previous..].as_ptr(),
                    self.voice_buffer[first_voice_index..].as_mut_ptr(),
                    max_voices,
                );
            }
        }
    }

    /// Updates the ages for each active voices.
    fn increment_active_ages(&mut self) {
        let seconds_per_sample = 1f32 / self.audio_ctx.sample_rate as f32;
        for active_voice in self.active_voices.iter_mut() {
            if let Some(active_voice) = active_voice {
                active_voice.age += seconds_per_sample;
            }
        }
    }

    /// Consumes the events at the current sample and updates the active voices.
    pub(super) fn consume_events_at_sample(&mut self, sample: usize) {
        // Increment ages for each active voices
        self.increment_active_ages();

        // Consume event and create events
    }
}
