use crate::{
    data_types::{PlaybackContext, Ticks},
    mixer::{Project, TrackID},
    track::error::TrackError,
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
    /// The track preparation failed for a specific track because of an error in the node graph.
    TrackPrepareFailed(TrackID, TrackError),
    /// A thread could not be spawned to an OS error.
    ThreadSpawnFailed(String),
    /// CPAL stream error has occured during playback.
    PlayStreamError(cpal::Error),
    /// An audio command failed, which means that it is likely that the audio thread is frozen or crashed.
    CommandFailed(AudioCommand),
}

unsafe impl Sync for AudioError {}
