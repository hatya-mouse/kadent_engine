use std::time::Duration;

pub mod data_types;
pub mod graph;
pub mod mixer;
pub mod node;
pub mod thread;
pub mod track;

/// Maximum supported number of channels for audio output.
pub const MAX_CHANNELS: usize = 64;
/// Number of events to be processed in a single frame.
pub const MAX_EVENTS: usize = 4;
/// Duration to wait for the audio thread to process commands and events.
pub const THREAD_WAIT_DURATION: Duration = Duration::from_millis(5);
