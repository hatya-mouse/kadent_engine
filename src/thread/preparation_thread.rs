use crate::{
    data_types::PlaybackContext,
    mixer::{Mixer, Project},
    thread::{AudioError, AudioResult},
};
use std::sync::{Arc, Mutex, atomic::AtomicBool, mpsc};

pub(super) fn spawn_preparation_thread(
    project_to_prepare: Arc<Mutex<Option<(Project, PlaybackContext)>>>,
    result_tx: mpsc::Sender<Result<AudioResult, AudioError>>,
    mixer_arc: Arc<Mutex<Option<Mixer>>>,
    should_terminate: Arc<AtomicBool>,
) {
    std::thread::spawn(move || {
        loop {
            if should_terminate.load(std::sync::atomic::Ordering::SeqCst) {
                // Break the loop if the audio thread has requested termination
                break;
            }

            let Ok(mut project_guard) = project_to_prepare.lock() else {
                // Break the loop if the lock is poisoned, which indicates that the audio thread has panicked
                break;
            };
            let Some((new_project, playback_ctx)) = project_guard.take() else {
                continue;
            };

            let (prepared_mixer, errors) = new_project.prepare(playback_ctx);
            for (track_id, graph_error) in errors {
                let _ = result_tx.send(Err(AudioError::TrackPrepareFailed(track_id, graph_error)));
            }

            let Ok(mut mixer_guard) = mixer_arc.lock() else {
                // Break the loop if the lock is poisoned, which indicates that the audio thread has panicked
                break;
            };
            mixer_guard.replace(prepared_mixer);
        }
    });
}
