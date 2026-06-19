use crate::{
    data_types::MidiEvent,
    thread::{AudioCommand, AudioError, AudioResult},
};
use std::sync::{Arc, atomic::AtomicUsize, mpsc};

/// A struct to communicate with the audio thread.
pub struct AudioThreadHandle {
    pub audio_command_tx: mpsc::Sender<AudioCommand>,
    pub midi_producer: ringbuf::HeapProd<MidiEvent>,
    pub result_rx: mpsc::Receiver<Result<AudioResult, AudioError>>,
    pub vu_consumer: ringbuf::HeapCons<f32>,
    pub playhead: Arc<AtomicUsize>,
}
