use druid::{widget::Controller, Widget, Data};

pub trait RotationData {
    fn get_rotation(&self) -> f64;
    fn set_rotation(&mut self, rotation: f64);
}

pub struct RotationController {
    rotation_step: f64,
}

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

impl<T: Data + RotationData, W: Widget<T>> Controller<T, W>  for RotationController {

}