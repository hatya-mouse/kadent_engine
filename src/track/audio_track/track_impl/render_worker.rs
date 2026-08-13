use crate::{
    MAX_CHANNELS, data_types::PlaybackContext, mixer::TempoMap, track::audio_track::AudioRegion,
};
use ringbuf::traits::{Observer, Producer};
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

pub(super) fn spawn_render_worker(
    mut producer: ringbuf::HeapProd<f32>,
    regions: Vec<AudioRegion>,
    tempo_map: TempoMap,
    playback_ctx: PlaybackContext,
    is_running: Arc<AtomicBool>,
    mut worker_playhead: usize,
) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        let buffer_len = MAX_CHANNELS * playback_ctx.buffer_size;
        let mut render_buf = vec![0.0; buffer_len];

        // Keep render worker running until the is_running flag is set to false
        while is_running.load(Ordering::Relaxed) {
            if producer.vacant_len() >= buffer_len {
                // Clear the render buffer before rendering
                render_buf.fill(0.0);
                // Render each region into the render buffer
                for region in &regions {
                    region.render_buffer(
                        worker_playhead,
                        &mut render_buf,
                        &tempo_map,
                        &playback_ctx,
                    );
                }

                // Push the rendered buffer into the ring buffer
                producer.push_slice(&render_buf);

                // Advance the worker playhead by the buffer size
                worker_playhead += playback_ctx.buffer_size;
            } else {
                std::thread::sleep(Duration::from_millis(2));
            }
        }
    })
}
