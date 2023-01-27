use druid::{widget::Controller, Data, Widget, Event};
pub trait ZoomData {
    fn get_zoom_scale(&self) -> f64;
    fn set_zoom_scale(&mut self, scale: f64);
}

pub struct ZoomController {
    min_zoom_scale: f64,
    max_zoom_scale: f64,
    zoom_step: f64,
}

impl ZoomController {
    pub fn new(min_zoom_scale: f64, max_zoom_scale: f64, zoom_step: f64) -> Self {
        Self { 
            min_zoom_scale,
            max_zoom_scale,
            zoom_step,
        }
    }
}

impl Default for ZoomController {
    fn default() -> Self {
        Self {
            min_zoom_scale: 0.2,
            max_zoom_scale: 1.5,
            zoom_step: 0.05,
        }
    }
}

impl<T: Data + ZoomData, W: Widget<T>> Controller<T, W> for ZoomController {
    fn event(&mut self, child: &mut W, ctx: &mut druid::EventCtx, event: &Event, data: &mut T, env: &druid::Env) {
        match event {
            Event::Wheel(wheel) if wheel.mods.ctrl() => {
                let mut current_zoom_scale = data.get_zoom_scale();
                if wheel.wheel_delta.y < 0.0 && current_zoom_scale < self.max_zoom_scale {
                     current_zoom_scale += self.zoom_step;
                    
                    if current_zoom_scale > self.max_zoom_scale {
                        current_zoom_scale = self.max_zoom_scale;
                    }
                } else if wheel.wheel_delta.y > 0.0 && current_zoom_scale > self.min_zoom_scale {
                    current_zoom_scale -= self.zoom_step;

                    if current_zoom_scale < self.min_zoom_scale {
                        current_zoom_scale = self.min_zoom_scale
                    }
                }
                data.set_zoom_scale(current_zoom_scale);
                // println!("Zoom scale: {:?}", current_zoom_scale);
            },

            _ => (),
        }
        child.event(ctx, event, data, env);


    }
}