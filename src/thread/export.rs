use crate::{
    data_types::PlaybackContext,
    mixer::Project,
    thread::{AudioError, AudioResult},
};
use std::{sync::mpsc, thread};

pub(super) fn spawn_export_thread(
    result_tx: mpsc::Sender<Result<AudioResult, AudioError>>,
    project: Project,
    playback_ctx: PlaybackContext,
) {
    thread::spawn(move || {
        let start_sample = project.tempo_map.ticks_to_samples(project.range_start);
        let end_sample = start_sample + project.tempo_map.ticks_to_samples(project.range_duration);
        let buffer_size = playback_ctx.buffer_size;
        let channels = playback_ctx.channels;

        let mut mixer = match project.prepare(playback_ctx) {
            Ok(mixer) => mixer,
            Err(err) => {
                result_tx.send(Err(AudioError::GraphError(err))).unwrap();
                return;
            }
        };

        mixer.seek(start_sample);

        let total_samples = (end_sample - start_sample) * channels;
        let mut output: Vec<f32> = Vec::with_capacity(total_samples);
        let mut buf = vec![0.0f32; buffer_size * channels];
        let mut playhead = start_sample;

        while playhead < end_sample {
            mixer.process(true, playhead, &mut buf);
            let frames = (end_sample - playhead).min(buffer_size);
            output.extend_from_slice(&buf[..frames * channels]);
            playhead += frames;
        }

        result_tx
            .send(Ok(AudioResult::ExportedAudio(output)))
            .unwrap();
    });
}
