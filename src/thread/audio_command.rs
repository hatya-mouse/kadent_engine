use crate::{
    data_types::Beats,
    graph::error::GraphError,
    mixer::{Project, TrackID},
};
use midir::MidiInputPort;

#[derive(Clone)]
pub enum AudioCommand {
    Play,
    Pause,
    Seek(Beats),
    UpdateProject(Box<Project>),
    ExportAudio(Box<Project>),
    ArmTrack(TrackID),
    DisarmTrack,
}

#[derive(Clone)]
pub enum MidiCommand {
    SetMidiPort(MidiInputPort),
    DisconnectMidiPort,
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
