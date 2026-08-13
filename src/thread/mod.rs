mod audio_command;
mod audio_thread;
mod export;
mod handle;
mod output_callback;
mod preparation_thread;

pub use audio_command::{AudioCommand, AudioError, AudioResult};
pub use handle::AudioThreadHandle;

use crate::data_types::{MidiEvent, PlaybackContext};
use ringbuf::{HeapRb, traits::Split};
use std::{
    sync::{
        Arc,
        atomic::{AtomicI64, AtomicU64},
        mpsc,
    },
    thread,
};

pub struct AudioThread;

impl AudioThread {
    pub fn spawn(
        playback_ctx: PlaybackContext,
    ) -> (AudioThreadHandle, ringbuf::HeapProd<MidiEvent>) {
        // MPSC channels to send commands to the processing threads from the host.
        let (audio_command_tx, audio_command_rx) = mpsc::channel();
        // MPSC channel to send the results back to the host.
        let (result_tx, result_rx) = mpsc::channel();
        // Shared playhead position using Arc and AtomicUsize for thread-safe access.
        let playhead = Arc::new(AtomicU64::new(0));
        let playhead_ticks = Arc::new(AtomicI64::new(0));
        let playhead_clone = playhead.clone();
        let playhead_ticks_clone = playhead_ticks.clone();
        // A ringbuf to send MIDI events to the audio thread from the midi thread.
        let (midi_producer, midi_consumer) = HeapRb::<MidiEvent>::new(64).split();
        // A ringbuf to send the calculated VU levels to the host.
        let (vu_producer, vu_consumer) = HeapRb::<f32>::new(playback_ctx.channels * 2).split();

        // --- MAIN AUDIO THREAD ---
        thread::spawn(move || {
            audio_thread::audio_thread(
                audio_command_rx,
                result_tx,
                midi_consumer,
                vu_producer,
                playhead_clone,
                playhead_ticks_clone,
                playback_ctx,
            );
        });

        (
            AudioThreadHandle {
                audio_command_tx,
                result_rx,
                vu_consumer,
                playhead,
                playhead_ticks,
            },
            midi_producer,
        )
    }
}
