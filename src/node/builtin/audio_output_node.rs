use std::ptr::copy_nonoverlapping;

use crate::{
    data_types::{AudioContext, PlaybackContext, TypeInfo},
    graph::error::NodeError,
    node::Node,
};

/// An empty node that just writes the `process` input to the node output.
#[derive(Default, Clone)]
pub struct AudioOutputNode {
    data_type: TypeInfo,
}

impl Node for AudioOutputNode {
    fn clone_box(&self) -> Box<dyn Node> {
        Box::new(self.clone())
    }

    fn get_input_names(&self) -> Vec<String> {
        vec!["audio".to_string()]
    }

    fn get_output_names(&self) -> Vec<String> {
        Vec::new()
    }

    fn get_input_len(&self) -> usize {
        1
    }

    fn get_output_len(&self) -> usize {
        0
    }

    fn get_input_type(&self, index: usize) -> Option<&TypeInfo> {
        if index == 0 {
            Some(&self.data_type)
        } else {
            None
        }
    }

    fn get_output_type(&self, _index: usize) -> Option<&TypeInfo> {
        None
    }

    fn update(&mut self, _audio_ctx: &AudioContext) {}

    fn prepare(&mut self, playback_ctx: &PlaybackContext) -> Result<(), Box<dyn NodeError>> {
        self.data_type = TypeInfo::new(4 * playback_ctx.channels * playback_ctx.buffer_size, 4);
        Ok(())
    }

    fn process(
        &mut self,
        inputs: &[*const u8],
        outputs: &[*mut u8],
        _playback_ctx: &PlaybackContext,
    ) {
        for (input, output) in inputs.iter().zip(outputs.iter()) {
            unsafe {
                // Write the input data to the output buffer
                // The buffer is of u8 type, so the size of the data type must be the same as the size of the buffer
                copy_nonoverlapping(*input, *output, self.data_type.size);
            }
        }
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
