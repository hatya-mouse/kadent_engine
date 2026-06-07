mod project;
mod tempo_event;
mod tempo_map;
mod track_id;

pub use project::Project;
pub use tempo_event::TempoEvent;
pub use tempo_map::TempoMap;
pub use track_id::TrackID;

pub struct Mixer {
    // --- PROJECT ---
    pub project: Project,
}

impl Mixer {
    // --- NEW ---

    /// Creates a new mixer instance with the given project.
    pub fn new(project: Project) -> Self {
        Self { project }
    }

    // --- PROJECT APPLYING ---

    /// Replaces the project with the new one. Tracks inside the project must have been prepared.
    pub fn apply_project(&mut self, new_project: Project, playhead: usize) {
        self.project = new_project;
        self.seek(playhead);
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
        unsafe {
            let len = self.project.audio_ctx.buffer_size * self.project.audio_ctx.channels;
            let dst = std::slice::from_raw_parts_mut(output.as_mut_ptr(), len);
            dst.fill(0.0);
        }

        // Call process function for every tracks
        for track in self.project.tracks.values_mut() {
            track.process_to_local_buffer(is_playing, playhead);
        }

        // Clamp the output between -1.0 and 1.0 for safety
        output.iter_mut().for_each(|s| *s = s.clamp(-1.0, 1.0))
    }
}
