mod project;
mod tempo_event;
mod tempo_map;
mod track_id;

pub use project::Project;
pub use tempo_event::TempoEvent;
pub use tempo_map::TempoMap;
pub use track_id::TrackID;

use crate::data_types::PlaybackContext;
use rayon::iter::{ParallelBridge, ParallelIterator};

pub struct Mixer {
    // --- PROJECT ---
    pub project: Project,
    // --- PLAYBACK CONTEXT ---
    pub playback_ctx: PlaybackContext,
}

impl Mixer {
    // --- NEW ---

    /// Creates a new mixer instance with the given project.
    pub fn new(project: Project, playback_ctx: PlaybackContext) -> Self {
        Self {
            project,
            playback_ctx,
        }
    }

    // --- SEEKING ---

    /// Tells every tracks that the it will seek.
    pub fn seek(&mut self, playhead: usize) {
        for track in self.project.tracks.values_mut() {
            track.seek(playhead);
        }
    }

    // --- MIXING PROCESS ---

    /// Processes the tracks in the mixer at the specified playhead.
    pub fn process(&mut self, is_playing: bool, playhead: usize, output: &mut [f32]) {
        // Fill the output buffer with zeros before processing
        output.iter_mut().for_each(|s| *s = 0.0);

        // Process samples and write them to local buffers
        self.project
            .tracks
            .values_mut()
            .par_bridge()
            .for_each(|track| {
                track.process_to_local_buffer(is_playing, playhead, &self.project.tempo_map);
            });

        // Add the output of each tracks to the main output buffer
        for track in self.project.tracks.values() {
            for (out_sample, track_sample) in output.iter_mut().zip(track.get_local_buffer()) {
                *out_sample += track_sample;
            }
        }

        // Clamp the output between -1.0 and 1.0 for safety
        output.iter_mut().for_each(|s| *s = s.clamp(-1.0, 1.0))
    }
}
