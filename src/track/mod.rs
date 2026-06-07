pub mod audio_track;
pub mod note_track;
mod region_id;

pub use region_id::RegionID;

use crate::{
    data_types::{AudioContext, Beats},
    graph::{Graph, error::GraphError},
    mixer::TempoMap,
};
use std::any::Any;

pub trait Track: Send + Any {
    /// Clones the track.
    fn clone_box(&self) -> Box<dyn Track>;

    /// Returns a reference to the Graph.
    fn get_graph(&self) -> &Graph;

    /// Returns a mutable reference to the Graph.
    fn get_graph_mut(&mut self) -> &mut Graph;

    /// Sets the Graph to the new one.
    fn set_graph(&mut self, graph: Graph);

    /// Moves the audio region to the new start beats.
    fn move_region(&mut self, region_id: &RegionID, new_start: Beats);

    /// Changes the duration of the region.
    fn set_region_duration(&mut self, region_id: &RegionID, new_duration: Beats);

    /// Removes the region from the track.
    fn remove_region(&mut self, region_id: &RegionID);

    /// Sets the audio context to the new one.
    fn set_audio_ctx(&mut self, audio_ctx: &AudioContext);

    /// Prepares for the seeking.
    fn seek(&mut self, playhead: usize);

    /// Prepares the track for processing.
    fn prepare(
        &mut self,
        start: usize,
        duration: usize,
        tempo_map: &TempoMap,
    ) -> Result<(), GraphError>;

    /// Processes the track and writes the processed output to the local buffer.
    fn process_to_local_buffer(&mut self, is_playing: bool, playhead: usize);

    /// Returns the processed audio data in the local buffer.
    fn get_local_buffer(&self) -> &[f32];

    /// Converts a reference to the track to any.
    fn as_any(&self) -> &dyn Any;

    /// Converts a mutable reference to the track to any.
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

impl Clone for Box<dyn Track> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}
