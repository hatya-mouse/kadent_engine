use crate::thread::{AudioCommand, AudioError, AudioResult};
use std::sync::{
    Arc,
    atomic::{AtomicI64, AtomicU64},
    mpsc,
};

/// A struct to communicate with the audio thread.
pub struct AudioThreadHandle {
    pub audio_command_tx: mpsc::Sender<AudioCommand>,
    pub result_rx: mpsc::Receiver<Result<AudioResult, AudioError>>,
    pub vu_consumer: ringbuf::HeapCons<f32>,
    pub playhead: Arc<AtomicU64>,
    pub playhead_tick: Arc<AtomicI64>,
}
