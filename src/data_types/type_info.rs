/// Represents the size and alignment of a type in memory.
/// `sample_size` represents the size of the value for a single sample, so you need to multiply it by the buffer size
/// to get the actual size of the buffer.
#[derive(Default, Clone, PartialEq, Eq, Debug)]
pub struct TypeInfo {
    pub sample_size: usize,
    pub align: usize,
}

impl TypeInfo {
    pub fn new(sample_size: usize, align: usize) -> Self {
        Self { sample_size, align }
    }

    /// Returns the actual size of the buffer, given the buffer size.
    pub fn actual_size(&self, buffer_size: usize) -> usize {
        self.sample_size * buffer_size
    }
}
