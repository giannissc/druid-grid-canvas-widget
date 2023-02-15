pub trait RotationData {
    fn get_rotation(&self) -> f64;
    fn set_rotation(&mut self, rotation: f64);
}

#[allow(dead_code)]
pub struct RotationController {
    rotation_step: f64,
}

#[allow(dead_code)]
impl RotationController {
    fn new(rotation_step: f64) -> Self {
        Self {
            rotation_step,
        }
    }
}

impl Default for RotationController {
    fn default() -> Self {
        Self {
            rotation_step: 0.1,
        }
    }
}