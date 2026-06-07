use crate::{
    data_types::{AudioContext, TypeInfo},
    graph::error::NodeError,
    node::Node,
};
use std::ptr::copy_nonoverlapping;

/// An empty node that just writes the `process` input to the node output.
#[derive(Default, Clone)]
pub struct AudioInputNode {
    data_type: TypeInfo,
}

impl Node for AudioInputNode {
    fn clone_box(&self) -> Box<dyn Node> {
        Box::new(self.clone())
    }

    fn get_input_names(&self) -> Vec<String> {
        Vec::new()
    }

    fn get_output_names(&self) -> Vec<String> {
        vec!["audio".to_string()]
    }

    fn get_input_len(&self) -> usize {
        0
    }

    fn get_output_len(&self) -> usize {
        1
    }

    fn get_input_type(&self, _index: usize) -> Option<&TypeInfo> {
        None
    }

    fn get_output_type(&self, index: usize) -> Option<&TypeInfo> {
        if index == 0 {
            Some(&self.data_type)
        } else {
            None
        }
    }

    fn update(&mut self, audio_ctx: &AudioContext) {
        self.data_type = TypeInfo::new(4 * audio_ctx.channels * audio_ctx.buffer_size, 4);
    }

    fn prepare(&mut self) -> Result<(), Box<dyn NodeError>> {
        Ok(())
    }

    fn process(&mut self, inputs: &[*const u8], outputs: &[*mut u8], _audio_ctx: &AudioContext) {
        for (input, output) in inputs.iter().zip(outputs.iter()) {
            unsafe {
                // Copy the entire input to the output
                // Divide by 4 because each sample is a 32-bit float (4 bytes)
                copy_nonoverlapping(*input, *output, self.data_type.size / 4);
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
