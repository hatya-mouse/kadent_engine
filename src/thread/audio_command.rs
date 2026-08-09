use crate::{
    data_types::{PlaybackContext, Ticks},
    graph::error::GraphError,
    mixer::{Project, TrackID},
};
use cpal::Device;

#[derive(Clone)]
pub enum AudioCommand {
    Play,
    Pause,
    Seek(Ticks),
    UpdateProject(Box<Project>),
    ExportAudio(Box<Project>, PlaybackContext),
    ArmTrack(TrackID),
    SetOutputDevice(Device),
    SetPlaybackCtx(PlaybackContext),
    DisarmTrack,
}

#[derive(Clone)]
pub enum AudioResult {
    ExportedAudio(Vec<f32>),
}

pub enum AudioError {
    GraphError(GraphError),
    PlayStreamError(cpal::Error),
    CommandFailed(AudioCommand),
}

unsafe impl Sync for AudioError {}
