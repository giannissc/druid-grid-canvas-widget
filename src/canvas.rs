//! A widget that allows for arbitrary layout of it's children.
use std::hash::Hash;

use druid::im::{HashMap,};
use druid::{
    BoxConstraints, Data, Env, Event, EventCtx, LayoutCtx, LifeCycle, LifeCycleCtx, PaintCtx, Size, UpdateCtx, Widget, WidgetPod, Point, WidgetId,};
///A container that allows for arbitrary layout.
///
///This widget allows you to lay widgets out at any point, and to allow that positioning to be dependent on the data.
///This is facilitated by the [`CanvasLayout`] trait, and will most typically be used by wrapping your desired widgets
///in a [`CanvasWrap`] wrapper.
///
///[`CanvasLayout`]: trait.CanvasLayout.html
///[`CanvasWrap`]: struct.CanvasWrap.html

//////////////////////////////////////////////////////////////////////////////////////////////////////
/// 
/// Canvas Widget
/// 
/////////////////////////////////////////////////////////////////////////////////////////////////////
#[allow(dead_code)]
pub struct Canvas<T>
{
    pub children: Vec<Child<T>>,
    pub position_map: HashMap<PointKey, usize>,
    pub offset: Point,
    pub scale: f64,
}


impl<T: Data> Default for Canvas<T>
{
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Canvas<T>
{
    pub fn new() -> Self {
        Self {
            children: vec![],
            position_map: HashMap::new(),
            offset: Point::ZERO,
            scale: 1.,
        }
    }
}

impl<T: Data> Widget<T> for Canvas<T>
{
    fn event(&mut self, _ctx: &mut EventCtx, _event: &Event, _data: &mut T, _env: &Env) {
        //we're letting their own filtering handle event filtering
        //we may want to revisit that decision
        // for child in self.children.iter_mut().filter_map(|x| x.widget_mut()) {
        //     child.event(ctx, event, data, env);
        // }
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &T, env: &Env) {
        if let LifeCycle::WidgetAdded = event {
            println!("Canvas received WidgetAdded");
            ctx.children_changed();
        }

        for child in self.children.iter_mut().filter_map(|x| x.widget_mut()) {
            child.lifecycle(ctx, event, data, env);
        }
    }

    fn update(&mut self, ctx: &mut UpdateCtx, _old_data: &T, data: &T, env: &Env) {
        for child in self.children.iter_mut().filter_map(|x| x.widget_mut()) {
            child.update(ctx, data, env);
        }
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, data: &T, env: &Env) -> Size {
        let mut temp = HashMap::new();

        for (index, child) in self.children.iter_mut().enumerate() {
            let (origin, _) = child.positioned_layout(ctx, data, env);
            let absolute_origin = (self.offset.to_vec2() / self.scale + origin.to_vec2());
            child.widget_mut().unwrap().set_origin(ctx, absolute_origin.to_point());
            temp.insert(origin.into(), index);
        }

        self.position_map = temp;

        //We always take the max size.
        bc.max()
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &T, env: &Env) {
        //TODO: filter painting based on our extents? (don't draw widgets entirely outside our bounds?)
        //It's the main reason we keep and update the rect
        for child in self.children.iter_mut() {
            child.widget_mut().unwrap().paint(ctx, data, env);
        }
    }
}

//////////////////////////////////////////////////////////////////////////////////////////////////////
/// 
/// Canvas Child Wrap
/// 
/////////////////////////////////////////////////////////////////////////////////////////////////////
pub enum Child<T> {
    Implicit {
        inner: WidgetPod<T, Box<dyn Widget<T>>>,
        closure: Box<dyn Fn(&T) -> Point>,
    },
    Explicit {
        inner: WidgetPod<T, Box<dyn Widget<T>>>,
        position: Point,
    }
}

impl<T: Data> Child<T> {
    fn widget_mut(&mut self) -> Option<&mut WidgetPod<T, Box<dyn Widget<T>>>> {
        match self {
            Child::Explicit { inner, ..} | Child::Implicit { inner, ..} => Some(inner),
        }
    }

    #[allow(dead_code)]
    fn widget(&self) -> Option<&WidgetPod<T, Box<dyn Widget<T>>>> {
        match self {
            Child::Explicit { inner, ..} | Child::Implicit { inner, ..} => Some(inner),
        }
    }

    fn positioned_layout(&mut self, ctx: &mut LayoutCtx, data: &T, env: &Env) -> (Point, Size) {
        match self {
            Child::Explicit { inner, position } => {
                let size = inner.layout(ctx, &BoxConstraints::new(Size::ZERO, Size::new(f64::INFINITY, f64::INFINITY)), data, env);
                (*position, size)
            },
            Child::Implicit { inner, closure } => {
                let desired_origin = (closure)(data);
                let desired_size = inner.layout(
                    ctx,
                    &BoxConstraints::new(Size::ZERO, Size::new(f64::INFINITY, f64::INFINITY)),
                    data,
                    env,
                );
        
                let origin: Point = desired_origin;
                inner.set_origin(ctx, origin);
                (desired_origin, desired_size)
            }
        }
        
    }
}

///////////////////////////////////////////////////////////////////////////////////////////////
/// 
/// PointKey
/// 
///////////////////////////////////////////////////////////////////////////////////////////////
#[derive(Debug, PartialEq, Hash, Eq, Clone)]
pub struct PointKey {
    /// The x coordinate.
    pub x: i32,
    /// The y coordinate.
    pub y: i32,
}

impl PointKey {
    pub fn new(x: i32, y: i32) -> Self {
        Self { x, y}
    }
}


impl Default for PointKey {
    fn default() -> Self {
        Self { x: 0, y: 0 }
    }
}

impl From<Point> for PointKey {
    fn from(value: Point) -> Self {
        Self {
            x: value.x as i32,
            y: value.y as i32,
        }
    }
}

impl Into<Point> for PointKey {
    fn into(self) -> Point {
        Point {
            x: self.x.into(),
            y: self.y.into(),
        }
    }
}