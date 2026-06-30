use crate::track::audio_track::AudioTrack;

impl AudioTrack {
    // --- LOCAL BUFFER ---

    pub(super) fn init_local_buffers(&mut self) {
        self.local_buffer = vec![
            0.0;
            self.hardware_config.buffer_size as usize
                * self.proj_config.channels as usize
        ];
    }
}
