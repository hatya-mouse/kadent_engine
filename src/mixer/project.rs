use crate::{
    data_types::{HardwareConfig, ProjectConfig, Ticks},
    graph::error::GraphError,
    mixer::{TempoMap, track_id::TrackID},
    track::Track,
};
use std::collections::HashMap;

#[derive(Clone)]
pub struct Project {
    // --- TRACKS ---
    /// Tracks in the project.
    pub tracks: HashMap<TrackID, Box<dyn Track>>,

    // --- TEMPO MAP ---
    /// A tempo map to store the tempo changes.
    pub tempo_map: TempoMap,

    // --- CONFIGURATIONS ---
    pub proj_config: ProjectConfig,
    pub hardware_config: HardwareConfig,

    // --- RANGE ---
    /// The start beats of the range to be exported or played.
    pub range_start: Ticks,
    /// The duration of the range to be exported or played.
    pub range_duration: Ticks,

    // --- MISCS ---
    /// The next track ID for generating track IDs.
    next_track_id: u64,
}

impl Project {
    // --- NEW ---

    /// Creates a new project with the specified initial bpm.
    pub fn new(
        bpm: f64,
        range_start: Ticks,
        range_duration: Ticks,
        proj_config: ProjectConfig,
        hardware_config: HardwareConfig,
    ) -> Self {
        Self {
            tracks: HashMap::new(),
            tempo_map: TempoMap::new(bpm, proj_config.clone(), hardware_config.clone()),
            proj_config,
            hardware_config,
            range_start,
            range_duration,
            next_track_id: 0,
        }
    }

    /// Creates a new project with the given tempo map.
    pub fn with_tempo_map(
        proj_config: ProjectConfig,
        hardware_config: HardwareConfig,
        tempo_map: TempoMap,
        range_start: Ticks,
        range_duration: Ticks,
    ) -> Self {
        Self {
            tracks: HashMap::new(),
            tempo_map,
            proj_config,
            hardware_config,
            range_start,
            range_duration,
            next_track_id: 0,
        }
    }

    // --- TRACK ID GENERATION ---

    /// Sets the next track ID for generating track IDs.
    pub fn set_next_track_id(&mut self, next_id: u64) {
        self.next_track_id = next_id;
    }

    /// Generates a new unique track ID.
    fn generate_track_id(&mut self) -> TrackID {
        let id = TrackID(self.next_track_id);
        self.next_track_id += 1;
        id
    }

    // --- TRACK MANAGEMENT ---

    /// Adds a new track to the mixer, setting the project context to the one in the mixer.
    pub fn add_track(&mut self, mut track: Box<dyn Track>) -> TrackID {
        let id = self.generate_track_id();
        track.set_config(&self.proj_config, &self.hardware_config);
        self.tracks.insert(id, track);
        id
    }

    /// Removes the track from the mixer.
    pub fn remove_track(&mut self, id: &TrackID) {
        self.tracks.remove(id);
    }

    /// Returns a reference to the track.
    pub fn get_track(&mut self, id: &TrackID) -> Option<&dyn Track> {
        self.tracks.get(id).map(|track| &**track)
    }

    /// Returns a mutable reference to the track.
    pub fn get_track_mut(&mut self, id: &TrackID) -> Option<&mut Box<dyn Track>> {
        self.tracks.get_mut(id)
    }

    // --- MIXING PREPARATION ---

    /// Prepares the tracks in the mixer for the playback.
    /// `start` and `duration` indicates the range to be processed.
    pub fn prepare(&mut self) -> Result<(), GraphError> {
        // Convert the start and duration beats to samples
        let start_samples = self.tempo_map.ticks_to_samples(self.range_start);
        let duration_samples = self.tempo_map.ticks_to_samples(self.range_duration);

        // Prepare the tracks one by one
        for track in self.tracks.values_mut() {
            track.prepare(start_samples, duration_samples, &self.tempo_map)?;
        }

        Ok(())
    }
}
