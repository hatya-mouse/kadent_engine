use serde::{Deserialize, Serialize};

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct AudioContext {
    pub resolution: u64,
    pub channels: usize,
    pub sample_rate: usize,
    pub buffer_size: usize,
    pub max_voices: usize,
}
