//! A widget that allows for arbitrary layout of it's children.
use std::hash::Hash;

use druid::im::HashMap;
use druid::im::hashmap::{IterMut, Iter};
use druid::kurbo::Rect;
use druid::{
    BoxConstraints, Data, Env, Event, EventCtx, LayoutCtx, LifeCycle, LifeCycleCtx, PaintCtx, Size, UpdateCtx, Widget, WidgetPod, Point,
};

///A container that allows for arbitrary layout.
///
///This widget allows you to lay widgets out at any point, and to allow that positioning to be dependent on the data.
///This is facilitated by the [`CanvasLayout`] trait, and will most typically be used by wrapping your desired widgets
///in a [`CanvasWrap`] wrapper.
///
///[`CanvasLayout`]: trait.CanvasLayout.html
///[`CanvasWrap`]: struct.CanvasWrap.html
pub struct Canvas<T: Data> where
(dyn Positioned<T> + 'static): Clone
{
    children: HashMap<RectInt, Box<dyn Positioned<T>>>,
}

impl<T: Data> Default for Canvas<T> where
(dyn Positioned<T> + 'static): Clone 
{
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Data> Canvas<T> where
(dyn Positioned<T> + 'static): Clone
{
    pub fn new() -> Self {
        Self {
            children: HashMap::new(),
        }
    }
    pub fn with_child(mut self, child: impl Positioned<T> + 'static) -> Self {
        self.children.insert(RectInt::ZERO, Box::new(child));
        self
    }

    pub fn add_child(&mut self, ctx: &mut EventCtx, child: impl Positioned<T> + 'static) {
        self.children.insert(RectInt::ZERO, Box::new(child));
        ctx.children_changed();
    }

    pub fn remove_child(&mut self, ctx: &mut EventCtx, from: RectInt, ){
        self.children.remove(&from);
        ctx.children_changed();
    }

    pub fn move_child(&mut self, ctx: &mut EventCtx, from: RectInt, to: RectInt){
        let child_from = self.children.remove(&from);
        if let Some(child_from) = child_from {
            self.children.insert(to, child_from);
        }
        ctx.children_changed();
    }

    pub fn exchange_child(&mut self, ctx: &mut EventCtx, from: RectInt, to: RectInt){
        let child_from = self.children.remove(&from);
        let child_to = self.children.remove(&to);
        if let (Some(child_from), Some(child_to)) = (child_from, child_to) {
            self.children.insert(to, child_from);
            self.children.insert(from, child_to);
        }

        ctx.children_changed();
    }

    pub fn children_mut(&mut self, ctx: &mut EventCtx) -> IterMut<RectInt, Box<dyn Positioned<T>>> {
        ctx.children_changed();
        self.children.iter_mut()
    }
    
    pub fn children(&self) -> Iter<RectInt, Box<dyn Positioned<T>>>{
        self.children.iter()
    }

}

impl<T: Data> Widget<T> for Canvas<T> where
(dyn Positioned<T> + 'static): Clone
{
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env) {
        //we're letting their own filtering handle event filtering
        //we may want to revisit that decision
        for (_, child) in self.children.iter_mut() {
            child.event(ctx, event, data, env);
        }
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &T, env: &Env) {
        for (_, child) in self.children.iter_mut() {
            child.lifecycle(ctx, event, data, env);
        }
    }

    fn update(&mut self, ctx: &mut UpdateCtx, old_data: &T, data: &T, env: &Env) {
        for (_, child) in self.children.iter_mut() {
            child.update(ctx, old_data, data, env);
        }
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, data: &T, env: &Env) -> Size {
        let mut temp = HashMap::new();
        for (_, child) in self.children.iter_mut() {
            let (origin, size) = child.positioned_layout(ctx, data, env);
            let origin: PointInt = origin.into();
            let size: SizeInt = size.into();
            temp.insert(RectInt::new(origin, size), child.clone());
        }

        self.children = temp;

        //We always take the max size.
        let size = bc.max();
        if size.width.is_infinite() {
            log::warn!("Infinite width passed to Canvas");
        }
        if size.height.is_infinite() {
            log::warn!("Infinite height passed to Canvas");
        }
        size
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &T, env: &Env) {
        //TODO: filter painting based on our extents? (don't draw widgets entirely outside our bounds?)
        //It's the main reason we keep and update the rect
        for (_, child) in self.children.iter_mut() {
            child.paint(ctx, data, env);
        }
    }
}

pub struct CanvasChildWrap<W: Widget<T>, T: Data, F: Fn(&T) -> Point> {
    inner: WidgetPod<T, W>,
    closure: F,
}
impl<W: Widget<T>, T: Data, F: Fn(&T) -> Point> CanvasChildWrap<W, T, F> {
    pub fn new(widget: W, closure: F) -> Self {
        Self {
            inner: WidgetPod::new(widget),
            closure,
        }
    }
}

impl<W: Widget<T>, T: Data, F: Fn(&T) -> Point> Positioned<T> for CanvasChildWrap<W, T, F> {
    fn positioned_layout(&mut self, ctx: &mut LayoutCtx, data: &T, env: &Env) -> (Point, Size) {
        let desired_origin = (self.closure)(data);
        let desired_size = self.inner.layout(
            ctx,
            &BoxConstraints::new(Size::ZERO, Size::new(f64::INFINITY, f64::INFINITY)),
            data,
            env,
        );
        println!("{:?} {:?}", desired_origin, desired_size);

        let point: Point = desired_origin.clone().into();
        self.inner.set_layout_rect(
            ctx,
            data,
            env,
            Rect::from_origin_size(point, desired_size),
        );
        (desired_origin, desired_size)
    }
}

impl<W: Widget<T>, T: Data, F: Fn(&T) -> Point> Widget<T> for CanvasChildWrap<W, T, F> {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env) {
        self.inner.event(ctx, event, data, env);
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &T, env: &Env) {
        self.inner.lifecycle(ctx, event, data, env);
    }

    fn update(&mut self, ctx: &mut UpdateCtx, old_data: &T, data: &T, env: &Env) {
        self.inner.update(ctx, data, env);
        if (self.closure)(data) != (self.closure)(old_data) {
            ctx.request_layout();
            //println!("Repaint requested");
        }
    }

    //NOTE: This is not called when we're being laid out on a canvas, so we act transparently.
    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, data: &T, env: &Env) -> Size {
        self.inner.layout(ctx, bc, data, env)
    }

    fn paint(&mut self, paint_ctx: &mut PaintCtx, data: &T, env: &Env) {
        self.inner.paint(paint_ctx, data, env);
    }
}

///
pub trait Positioned<T: Data>: Widget<T> {
    fn positioned_layout(&mut self, ctx: &mut LayoutCtx, data: &T, env: &Env) -> (Point, Size);
}

///////////////////////////////////////////////////////////////////////////////////////////////
/// 
/// 
/// 
///////////////////////////////////////////////////////////////////////////////////////////////
#[derive(Debug, PartialEq, Hash, Eq, Clone)]
pub struct RectInt {
    position: PointInt,
    size: SizeInt,
}

impl RectInt {
    pub const ZERO: RectInt = RectInt {
        position: PointInt { x: 0, y: 0 },
        size: SizeInt { width: 0, height: 0},
    };
    
    pub fn new(position: PointInt, size: SizeInt) -> Self {
        Self {
            position,
            size,
        }
        

    }
}

#[derive(Debug, PartialEq, Hash, Eq, Clone)]
pub struct PointInt {
    /// The x coordinate.
    pub x: i32,
    /// The y coordinate.
    pub y: i32,
}

impl PointInt {
    pub fn new(x: i32, y: i32) -> Self {
        Self { x, y}
    }
}


impl Default for PointInt {
    fn default() -> Self {
        Self { x: 0, y: 0 }
    }
}

impl From<Point> for PointInt {
    fn from(value: Point) -> Self {
        Self {
            x: value.x as i32,
            y: value.y as i32,
        }
    }
}

impl Into<Point> for PointInt {
    fn into(self) -> Point {
        Point {
            x: self.x.into(),
            y: self.y.into(),
        }
    }
}

#[derive(Debug, PartialEq, Hash, Eq, Clone)]
pub struct SizeInt {
    /// The width.
    pub width: u32,
    /// The height.
    pub height: u32,
}

impl SizeInt {
    fn new(width: u32, height: u32) -> Self {
        Self { width, height}
    }
}

impl Default for SizeInt {
    fn default() -> Self {
        Self { width: 0, height: 0 }
    }
}

impl From<Size> for SizeInt {
    fn from(value: Size) -> Self {
        Self {
            width: value.width as u32,
            height: value.height as u32,
        }
    }
}

impl Into<Size> for SizeInt {
    fn into(self) -> Size {
        Size {
            width: self.width.into(),
            height: self.height.into(),
        }
    }
}

