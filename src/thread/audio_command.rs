use cpal::Device;

use crate::{
    data_types::Beats,
    graph::error::GraphError,
    mixer::{Project, TrackID},
};

#[derive(Clone)]
pub enum AudioCommand {
    Play,
    Pause,
    Seek(Beats),
    UpdateProject(Box<Project>),
    ExportAudio(Box<Project>),
    ArmTrack(TrackID),
    SetOutputDevice(Device),
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
