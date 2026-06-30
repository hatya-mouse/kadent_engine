use cpal::{SupportedBufferSize, traits::DeviceTrait};
use serde::{Deserialize, Serialize};

/// Configurations related to the output device.
#[derive(Clone, Default, Serialize, Deserialize, Debug)]
pub struct HardwareConfig {
    pub sample_rate: u64,
    pub buffer_size: u32,
    pub max_voices: u16,
}

impl HardwareConfig {
    pub fn fallback_config() -> Self {
        HardwareConfig {
            sample_rate: 48000,
            buffer_size: 512,
            max_voices: 32,
        }
    }

    pub fn from_output_device(device: &cpal::Device) -> Option<Self> {
        if let Ok(mut supported_configs) = device.supported_output_configs()
            && let Some(config_range) = supported_configs.next()
        {
            let buffer_size = match config_range.buffer_size() {
                SupportedBufferSize::Range { min: _, max } => *max,
                _ => 512,
            };

            return Some(HardwareConfig {
                sample_rate: config_range.max_sample_rate() as u64,
                buffer_size,
                max_voices: 32,
            });
        }

        None
    }
}
