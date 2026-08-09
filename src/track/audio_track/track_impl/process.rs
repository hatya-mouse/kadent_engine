use crate::{data_types::PlaybackContext, track::audio_track::AudioTrack};

impl AudioTrack {
    // --- LOCAL BUFFER ---

    pub(super) fn init_local_buffers(&mut self, playback_ctx: &PlaybackContext) {
        self.local_buffer = vec![0.0; playback_ctx.buffer_size * playback_ctx.channels];
    }
}
