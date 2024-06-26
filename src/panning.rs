///////////////////////////////////////////////////////////////////////////////////////////////////
///
/// Imports
///
///////////////////////////////////////////////////////////////////////////////////////////////////
use druid::{widget::Controller, Data, Event, Lens, Point, Vec2, Widget};
use log::debug;

///////////////////////////////////////////////////////////////////////////////////////////////////
///
/// PanningData
///
///////////////////////////////////////////////////////////////////////////////////////////////////
pub trait PanDataAccess {
    fn get_offset(&self) -> Point;
    fn set_offset(&mut self, offset: Point);
}

#[derive(Clone, Data, Lens, PartialEq, Debug)]
pub struct PanData
where
    PanData: PanDataAccess,
{
    pub offset: Point,
}

impl PanData {
    pub fn new() -> Self {
        Self {
            offset: Point::new(0.0, 0.0),
        }
    }
}

impl PanDataAccess for PanData {
    fn get_offset(&self) -> Point {
        self.offset
    }

    fn set_offset(&mut self, offset: Point) {
        self.offset = offset;
    }
}

///////////////////////////////////////////////////////////////////////////////////////////////////
///
/// Panning Controller
///
///////////////////////////////////////////////////////////////////////////////////////////////////
pub struct PanController {
    start_mouse_position: Option<Point>,
    previous_mouse_position: Option<Point>,
    start_offset: Point,
    min_offset: Point,
    max_offset: Point,
}

impl PanController {
    pub fn new(min_offset: Point, max_offset: Point) -> Self {
        PanController {
            start_mouse_position: None,
            previous_mouse_position: None,
            start_offset: Point::new(0.0, 0.0),
            min_offset,
            max_offset,
        }
    }
}

impl Default for PanController {
    fn default() -> Self {
        Self {
            start_mouse_position: None,
            previous_mouse_position: None,
            start_offset: Point::new(0.0, 0.0),
            min_offset: Point::new(f64::NEG_INFINITY, f64::NEG_INFINITY),
            max_offset: Point::new(f64::INFINITY, f64::INFINITY),
        }
    }
}

impl<T: Data + PanDataAccess, W: Widget<T>> Controller<T, W> for PanController {
    fn event(
        &mut self,
        child: &mut W,
        ctx: &mut druid::EventCtx,
        event: &druid::Event,
        data: &mut T,
        env: &druid::Env,
    ) {
        child.event(ctx, event, data, env);

        if ctx.is_handled() {
            return;
        }

        let mut release_delta = Vec2::new(0.0, 0.0);

        match event {
            Event::MouseDown(mouse_event) => {
                if mouse_event.button.is_middle() {
                    self.start_mouse_position = Some(mouse_event.window_pos);
                    self.previous_mouse_position = Some(mouse_event.window_pos);
                    // self.start_offset = data.absolute_offset;
                    self.start_offset = data.get_offset();
                    debug!("Start offset: {:?}", self.start_offset);
                    ctx.set_active(true);
                    ctx.request_focus();
                }
            }
            Event::MouseMove(mouse_event) => {
                if let (Some(start_mouse_position), Some(previous_mouse_position)) =
                    (self.start_mouse_position, self.previous_mouse_position)
                {
                    // Calculate delta from current position
                    release_delta = mouse_event.window_pos - start_mouse_position;
                    let mut offset = self.start_offset + release_delta;

                    self.previous_mouse_position = Some(mouse_event.window_pos);

                    if offset.x > self.max_offset.x {
                        offset.x = self.max_offset.x;
                    } else if offset.x < self.min_offset.x {
                        offset.x = self.min_offset.x;
                    }

                    if offset.y > self.max_offset.y {
                        offset.y = self.max_offset.y;
                    } else if offset.y < self.min_offset.y {
                        offset.y = self.min_offset.y;
                    }

                    // data.absolute_offset = offset;
                    data.set_offset(offset);
                    ctx.set_handled();
                    // debug!("Current delta: {:?}", data.relative_offset);
                }
            }
            Event::MouseUp(mouse_event) => {
                if mouse_event.button.is_middle() {
                    ctx.set_active(false);
                    ctx.resign_focus();
                    self.start_mouse_position = None;
                    // debug!("Finish offset: {:?}", data.absolute_offset);
                    debug!("Release delta: {:?}\n", release_delta);
                }
            }
            _ => {}
        }
    }
}
