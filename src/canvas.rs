//! A widget that allows for arbitrary layout of it's children.
use std::hash::Hash;

use druid::im::HashMap;
use druid::kurbo::Rect;
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
    children: Vec<Child<T>>,
    position_map: HashMap<PointKey, usize>,
    id_map: HashMap<WidgetId, usize>,
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
            id_map: HashMap::new(),
        }
    }

    // For index based layout containers the position will be replaced by an index
    // Might need two variants for this: add and add_relocate in case you don't want 
    // to remove the the exist at the to position. Useful for drag and drop between
    // different containers
    // A third method
    pub fn add_child(&mut self, child: impl Widget<T> + 'static, from: PointKey) {
        let index = self.position_map.remove(&from);
        
        if let Some(index) = index {
            self.children.remove(index);
        }
        let inner: WidgetPod<T, Box<dyn Widget<T>>> = WidgetPod::new(Box::new(child));
        let index = self.children.len();
        self.children.insert(index, Child::Explicit { inner, position: from.clone().into()});
        self.position_map.insert(from, index);

    }

    // For index based layout containers the position will be replaced by an index
    pub fn remove_child(&mut self, from: PointKey){
        // Swap item at index with last item and then delete 
        let delete_index = self.position_map.remove(&from);
        let last_index = self.children.len() - 1;
        if let Some(delete_index) = delete_index {
            let child = self.children.remove(last_index);
            if last_index != delete_index {
                // Update position map
                if let Child::Explicit {position, ..} = &child {
                    let key: PointKey = <Point as Into<PointKey>>::into(*position);
                    self.position_map.remove(&key);
                    self.position_map.insert(key, delete_index);
                }
                self.children.remove(delete_index);
                self.children.insert(delete_index, child); 
                // self.children.remove(index);
            }
        }
    }

    // For index based layout containers the position will be replaced by an index
    // Might need two variants for this: move and move_relocate in case you don't want 
    // to remove the the exist at the to position. Useful for drag and drop within the 
    // same container
    pub fn move_child(&mut self, from: PointKey, to: PointKey){
        let index_from = self.position_map.remove(&from);
        let index_to = self.position_map.remove(&to);
        
        if let Some(index) = index_to {
            self.children.remove(index);
        }

        if let Some(old_index) = index_from {
            let inner = self.children.remove(old_index);
            match inner {
                Child::Explicit { inner, ..} => {
                    let index = self.children.len();
                    self.children.insert(index, Child::Explicit { inner, position: to.clone().into()});
                    self.position_map.insert(from, index);
                },
                _ => (),
            }
        }
    }

    // For index based layout containers the position will be replaced by an index
    // Can be useful for drag and drop operations within the same container
    pub fn exchange_child(&mut self, from: PointKey, to: PointKey){
        let index_from = self.position_map.remove(&from);
        let index_to = self.position_map.remove(&to);
        if let (Some(index_from), Some(index_to)) = (index_from, index_to) {
            self.position_map.insert(to, index_from);
            self.position_map.insert(from, index_to);
        }
    }

    pub fn children_len(&self) -> usize {
        self.children.len()
    }
    // pub fn children_mut(&mut self, ctx: &mut EventCtx) -> &mut Vec<WidgetPod<T, Box<dyn Widget<T>>>> {
    //     ctx.children_changed();
    //     self.children.borrow_mut()
    // }
    
    // pub fn children(&self) -> &Vec<WidgetPod<T, Box<dyn Widget<T>>>> {
    //     self.children.borrow()
    // }

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
            child.widget_mut().unwrap().set_origin(ctx, data, env, origin);
            temp.insert(origin.into(), index);
        }

        self.position_map = temp;

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
        for child in self.children.iter_mut().filter_map(|x| x.widget_mut()) {
            child.paint(ctx, data, env);
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
        
                let point: Point = desired_origin;
                inner.set_layout_rect(
                    ctx,
                    data,
                    env,
                    Rect::from_origin_size(point, desired_size),
                );
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