/// An automation cursor that keeps track of the current index in a list of gradient values.
/// This is solely used to interpolate between **float** keyframes, not integer or boolean keyframes.
#[derive(Debug, Clone, Default)]
pub(super) struct FloatAutomationCursor {
    pub current_index: Option<usize>,
}

impl FloatAutomationCursor {
    pub(super) fn clear_cache(&mut self) {
        self.current_index = None;
    }

    pub(super) fn get_gradient_at(
        &mut self,
        gradient_vals: &[(usize, f32)],
        target_sample: usize,
    ) -> f32 {
        if gradient_vals.is_empty() {
            return 0.0;
        }

        if let Some(current_index) = self.current_index {
            let (current_end_sample, current_gradient) = gradient_vals[current_index];

            // Get the start sample of the current gradient range based on the end index of the previous range
            // ...|<-- previous gradient range -->|<-- current gradient range -->|
            //               current_start_sample ^           current_end_sample ^
            let current_start_sample = if current_index > 0 {
                gradient_vals[current_index - 1].0
            } else {
                0
            };

            // If the target sample is within the current gradient range, return the current gradient value
            //                ...-->|<-|-- current gradient range -->|<--...
            // current_start_sample ^  ^ target_sample               ^ current_end_sample
            if target_sample >= current_start_sample && target_sample <= current_end_sample {
                return current_gradient;
            }
        }

        // If the target sample is greater than the last gradient range, return 0.0
        // ...|<-- the last gradient range -->|<-- use zero as the gradient value...
        if let Some((last_end_sample, _)) = gradient_vals.last()
            && target_sample > *last_end_sample
        {
            return 0.0;
        }

        // Recalculate the gradient index and get the corresponding gradient value if the target sample is outside the current range
        self.calculate_grad_index_and_get_gradient(gradient_vals, target_sample)
    }

    /// Calculates the gradient index for the given target sample and returns the corresponding gradient value.
    /// Uses the newly calculated index to get the gradient value, or return 0.0 if no new index was found.
    fn calculate_grad_index_and_get_gradient(
        &mut self,
        gradient_vals: &[(usize, f32)],
        target_sample: usize,
    ) -> f32 {
        self.calculate_grad_index(gradient_vals, target_sample);

        if let Some(new_index) = self.current_index {
            let (_, new_gradient) = gradient_vals[new_index];
            new_gradient
        } else {
            0.0
        }
    }

    /// Calculates the gradient index for the given target sample.
    pub(super) fn calculate_grad_index(
        &mut self,
        gradient_vals: &[(usize, f32)],
        target_sample: usize,
    ) {
        if gradient_vals.is_empty() {
            self.current_index = None;
            return;
        }

        self.current_index = match gradient_vals
            .binary_search_by(|(end_sample, _)| end_sample.cmp(&target_sample))
        {
            // If the target sample is exactly equal to an end_sample of some gradient range, we can use that index directly
            Ok(idx) => Some(idx),
            Err(idx) => {
                if idx < gradient_vals.len() {
                    Some(idx)
                } else {
                    // When the target sample is greater than the last gradient range, use None
                    None
                }
            }
        };
    }
}
