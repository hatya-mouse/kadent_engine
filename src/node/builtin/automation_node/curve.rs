use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum CurveType {
    /// Linear interpolation between keyframes.
    Linear,
    /// Step interpolation between keyframes.
    Step,
    /// Smooth interpolation between keyframes with the given tension.
    Smooth { tension: f32 },
}

impl CurveType {
    #[inline(always)]
    pub fn evaluate_curve(&self, x: f32) -> f32 {
        match self {
            // Keep the value constantly until the next keyframe
            CurveType::Step => 0.0,
            // Linear interpolation between keyframes
            CurveType::Linear => x,
            // Smooth interpolation between keyframes with the given tension
            CurveType::Smooth { tension } => {
                if tension.abs() < 1e-4 {
                    // If the tension if small enough, just return the linear interpolation value
                    x
                } else {
                    (x * (1.0 + tension)) / (x * tension + 1.0)
                }
            }
        }
    }

    pub fn all() -> &'static [CurveType] {
        &[
            CurveType::Linear,
            CurveType::Step,
            CurveType::Smooth { tension: 1.0 },
        ]
    }
}
