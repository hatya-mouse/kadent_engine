mod audio_command;
mod audio_thread;
mod export;
mod handle;
mod output_callback;

pub use audio_command::{AudioCommand, AudioError, AudioResult};
pub use handle::AudioThreadHandle;

use crate::{data_types::MidiEvent, mixer::Project};
use ringbuf::{HeapRb, traits::Split};
use std::{
    sync::{Arc, atomic::AtomicUsize, mpsc},
    thread,
};

pub struct AudioThread;

impl AudioThread {
    pub fn spawn(
        mut initial_project: Project,
    ) -> (AudioThreadHandle, ringbuf::HeapProd<MidiEvent>) {
        // MPSC channels to send commands to the processing threads from the host.
        let (audio_command_tx, audio_command_rx) = mpsc::channel();
        // MPSC channel to send the results back to the host.
        let (result_tx, result_rx) = mpsc::channel();
        // Shared playhead position using Arc and AtomicUsize for thread-safe access.
        let playhead = Arc::new(AtomicUsize::new(0));
        let playhead_clone = playhead.clone();
        // A ringbuf to send MIDI events to the audio thread from the midi thread.
        let (midi_producer, midi_consumer) = HeapRb::<MidiEvent>::new(64).split();
        // A ringbuf to send the calculated VU levels to the host.
        let (vu_producer, vu_consumer) =
            HeapRb::<f32>::new(initial_project.proj_config.channels as usize * 2).split();

        // --- MAIN AUDIO THREAD ---
        thread::spawn(move || {
            // Prepare the initial project
            if let Err(err) = initial_project.prepare() {
                result_tx.send(Err(AudioError::GraphError(err))).unwrap();
            }

            audio_thread::audio_thread(
                audio_command_rx,
                result_tx,
                midi_consumer,
                vu_producer,
                playhead_clone,
                initial_project,
            );
        });

        (
            AudioThreadHandle {
                audio_command_tx,
                result_rx,
                vu_consumer,
                playhead,
            },
            midi_producer,
        )
    }
}
