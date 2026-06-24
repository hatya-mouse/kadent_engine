use crate::{
    mixer::{Mixer, Project},
    thread::{AudioError, AudioResult},
};
use std::{sync::mpsc, thread};

pub(super) fn spawn_export_thread(
    result_tx: mpsc::Sender<Result<AudioResult, AudioError>>,
    mut project: Project,
) {
    thread::spawn(move || {
        if let Err(err) = project.prepare() {
            result_tx.send(Err(AudioError::GraphError(err))).unwrap();
            return;
        }

        let start_sample = project.tempo_map.ticks_to_samples(project.range_start);
        let end_sample = start_sample + project.tempo_map.ticks_to_samples(project.range_duration);
        let buffer_size = project.audio_ctx.buffer_size;
        let channels = project.audio_ctx.channels;

        let mut mixer = Mixer::new(project);
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
