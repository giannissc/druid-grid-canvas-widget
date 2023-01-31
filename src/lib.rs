use std::hash::Hash;
use std::marker::PhantomData;
use druid::kurbo::Circle;
use druid::{BoxConstraints, Data, Env, Event, EventCtx, LayoutCtx, Lens, LifeCycle,
    LifeCycleCtx, PaintCtx, RenderContext, UpdateCtx, Widget, Selector, Point, Rect, Size, Color, MouseButton, WidgetPod, Affine,};

use druid::im::{HashMap, Vector, HashSet};

use druid_color_thesaurus::white;
use log::info;
use panning::PanningData;
use save_system::{SaveSystemData};
use zooming::ZoomData;

pub mod panning;
pub mod zooming;
pub mod snapping;
pub mod rotation;
pub mod canvas;
pub mod save_system;

//////////////////////////////////////////////////////////////////////////////////////////////////////
/// 
/// Command Selectors
/// 
/////////////////////////////////////////////////////////////////////////////////////////////////////
pub const SET_DISABLED: Selector = Selector::new("disabled-grid-state");
pub const SET_ENABLED: Selector = Selector::new("idle-grid-state");
pub const UPDATE_GRID_PLAYBACK: Selector = Selector::new("update-grid-playback");
pub const UPDATE_PAINT_PLAYBACK: Selector = Selector::new("update-paint-playback");
pub const REQUEST_PAINT: Selector =  Selector::new("request-paint");

//////////////////////////////////////////////////////////////////////////////////////////////////////
/// 
/// GridIndex
/// 
/////////////////////////////////////////////////////////////////////////////////////////////////////
#[derive(Clone, Data, Copy, PartialEq, Debug, Hash, Eq)]
pub struct GridIndex {
    pub row: usize,
    pub col: usize,
}

impl GridIndex {
    pub fn above(self) -> GridIndex {
        GridIndex {
            row: self.row - 1,
            col: self.col,
        }
    }

    pub fn below(self) -> GridIndex {
        GridIndex {
            row: self.row + 1,
            col: self.col,
        }
    }

    pub fn left(self) -> GridIndex {
        GridIndex {
            row: self.row,
            col: self.col - 1,
        }
    }

    pub fn right(self) -> GridIndex {
        GridIndex {
            row: self.row,
            col: self.col + 1,
        }
    }

    // Also known in vlsi as the Manhattan Architecture
    pub fn neighbors_rectilinear(self) -> [GridIndex; 4] {
        let above = self.above();
        let below = self.below();
        let left = self.left();
        let right = self.right();
        [above, below, left, right]
    }

    // Also known in vlsi as the X Architecture
    pub fn neighbors_diagonal(self) -> [GridIndex; 4] {
        let above = self.above();
        let below = self.below();
        let above_left = above.left();
        let above_right = above.right();
        let below_left = below.left();
        let below_right = below.right();
        [
            above_left,
            above_right,
            below_left,
            below_right,
        ]
    }
}

//////////////////////////////////////////////////////////////////////////////////////
//
// GridRunner
//
//////////////////////////////////////////////////////////////////////////////////////
pub trait GridItem: Copy + Clone + Hash + Eq{
    fn can_add(&self, other: Option<&Self>) -> bool;
    fn can_remove(&self) -> bool;
    fn can_move(&self, other: Option<&Self>) -> bool;
    fn get_color(&self) -> &Color;
}

//////////////////////////////////////////////////////////////////////////////////////
//
// GridState
//
//////////////////////////////////////////////////////////////////////////////////////
/// 
#[derive(Clone, Copy, PartialEq, Data, Debug)]
pub enum GridState{
    Idle,
    Running(GridAction),
    Disabled,
}

//////////////////////////////////////////////////////////////////////////////////////
//
// GridAction
//
//////////////////////////////////////////////////////////////////////////////////////

#[derive(Clone, Copy, PartialEq, Data, Debug)]
pub enum GridAction{
    Dynamic,
    Add,
    Remove,
    Move,
}

//////////////////////////////////////////////////////////////////////////////////////
//
// StackItem
//
//////////////////////////////////////////////////////////////////////////////////////
#[derive(Clone, PartialEq, Data, Debug, Hash, Eq)]
pub enum StackItem<T: GridItem>{
    Add(GridIndex, T, Option<T>),
    Remove(GridIndex, T),
    Move(GridIndex, GridIndex, T),
    BatchAdd(HashMap<GridIndex, (T, Option<T>)>),
    BatchRemove(HashMap<GridIndex, T>),
}

impl<T: GridItem> StackItem<T>{
    fn get_positions(&self) -> HashSet<GridIndex>{
        let mut set: HashSet<GridIndex> = HashSet::new();
        
        match self{
            StackItem::Add(pos, _, _) => {set.insert(*pos);},
            StackItem::Remove(pos, _) => {set.insert(*pos);},
            StackItem::Move(from, to , _) => {
                set.insert(*from);
                set.insert(*to);
            },
            StackItem::BatchAdd(item) => {
                for pos in item.keys(){
                    set.insert(*pos);
                }
            },
            StackItem::BatchRemove(item) => {
                for pos in item.keys(){
                    set.insert(*pos);
                }
            },
        }
        set
    }

    fn forward(&self, grid: &mut HashMap<GridIndex, T>){
        match self{
            StackItem::Add(pos, current_item, _) => {grid.insert(*pos, *current_item);},
            StackItem::Remove(pos, _) => {grid.remove(pos);},
            StackItem::Move(from, to, item) => {
                grid.remove(from);
                grid.insert(*to, *item);
            },
            StackItem::BatchAdd(items) => {
                for (key, (current_item, _)) in items {
                    grid.insert(*key, *current_item);
                }
            },
            StackItem::BatchRemove(items) => {
                for (key, _) in items {
                    grid.remove(key);
                }
            }
        }
    }

    fn reverse(&self, grid: &mut HashMap<GridIndex, T>){
        match self{
            StackItem::Add(pos, _, previous_item) => {
                grid.remove(pos);
                if let Some(previous_node) = previous_item {
                    grid.insert(*pos, *previous_node);
                }
            },
            StackItem::Remove(pos, item) => {grid.insert(*pos, *item);},
            StackItem::Move(from, to, item) => {
                grid.remove(to);
                grid.insert(*from, *item);
            }
            StackItem::BatchAdd(items) => {
                for (pos, (_, previous_item)) in items {
                    grid.remove(pos);
                    if let Some(previous_node) = previous_item {
                        grid.insert(*pos, *previous_node);
                    }
                }
            },
            StackItem::BatchRemove(items) => {
                for (pos, item) in items {
                    grid.insert(*pos, *item);
                }
            }
        }
    }
}

//////////////////////////////////////////////////////////////////////////////////////
//
// GridWidgetData
//
//////////////////////////////////////////////////////////////////////////////////////
#[derive(Clone, PartialEq, Data, Lens)]
pub struct GridWidgetData<T:GridItem + PartialEq>{
    pub grid: HashMap<GridIndex, T>,
    pub save_system: SaveSystemData<StackItem<T>>,
    pub action: GridAction,
    pub grid_item: T,
}

impl<T:GridItem + PartialEq> GridWidgetData<T>{
    pub fn new(initial_node: T) -> Self {
        GridWidgetData {
            grid: HashMap::new(),
            save_system: SaveSystemData::new(),
            action: GridAction::Dynamic,
            grid_item: initial_node,
        }
    }

    fn add_node(&mut self, pos: &GridIndex, item: T) -> bool{
        let option = self.grid.get(pos);

        let command_item;
        if option.is_none() {
            command_item = StackItem::Add(*pos, item, None);
        } else {
            command_item = StackItem::Add(*pos, item, Some(*option.unwrap()));
        }
        
        if item.can_add(option){
            self.grid.insert(*pos, item);
            self.save_system.submit_play(command_item);
            return true;
        }
        false
    }

    fn remove_node(&mut self, pos: &GridIndex) -> bool{
        if let Some(item) = self.grid.remove(pos){
            if item.can_remove(){
                let command_item = StackItem::Remove(*pos, item);
                self.save_system.submit_play(command_item);
                return true;
            } else {
                self.grid.insert(*pos, item);
            }
        }
        false
    }

    fn move_node(&mut self, from: &GridIndex, to:&GridIndex) -> bool{
        let item = self.grid.get(from).unwrap();
        let other = self.grid.get(to);
        if item.can_move(other) {
            let item = self.grid.remove(from).unwrap();
            self.grid.insert(*to, item);
            let command_item = StackItem::Move(*from, *to, item);
            self.save_system.submit_play(command_item);
            return true;
        }
        false
    }

    pub fn add_node_perimeter(
        &mut self,
        pos: GridIndex,
        row_n: usize,
        column_n: usize,
        tool: T,
    ) {
        let mut map: HashMap<GridIndex, (T, Option<T>)> = HashMap::new();
        for row in pos.row..pos.row + row_n {
            //debug!("Add node perimeter");
            //debug!("Row: {:?}", row);
            if row == pos.row || row == pos.row + row_n - 1 {
                // Top and Bottom Boundaries
                //debug!("Printing top/bottom boundary");
                for column in pos.col..pos.col + column_n {
                    map.insert(
                        GridIndex {
                            row: row,
                            col: column,
                        },
                        (tool, None),
                    );
                }
            } else {
                //debug!("Printing left/right boundary");
                // Left Boundary
                map.insert(
                    GridIndex {
                        row: row,
                        col: pos.col,
                    },
                    (tool, None),
                );
                // Right Boundary
                map.insert(
                    GridIndex {
                        row: row,
                        col: pos.col + column_n - 1,
                    },
                    (tool, None),
                );
            }
        }
        
        for (pos, (current_item, _)) in &map{
            self.grid.insert(*pos, *current_item);
        }
        self.save_system.submit_play(StackItem::BatchAdd(map))

    }

    pub fn submit_to_stack(&mut self, list: Vector<StackItem<T>>) {
        let mut val_list = Vector::new();
        
        for stack_item in list {
            match stack_item {
                StackItem::Add(pos, current_item, _) => {
                    let option = self.grid.get(&pos);
                    if current_item.can_add(option) {val_list.push_back(stack_item)}
                },
                StackItem::BatchAdd(mut map) => {
                    map.retain(|pos, (current_item, _)|{
                        let option = self.grid.get(pos);
                        current_item.can_add(option)
                    });

                    if !map.is_empty(){

                        val_list.push_back(StackItem::BatchAdd(map));
                    }
                },
                _ => (),
            }
        }
        self.save_system.append(val_list);
    }

    pub fn get_grid(&self) -> &HashMap<GridIndex, T> {
        &self.grid
    }

    pub fn clear_all(&mut self){
        self.save_system.submit_play(StackItem::BatchRemove(self.grid.clone()));
        self.grid.clear();
    }

    pub fn clear_except(&mut self, set: HashSet<T>){
        let mut map: HashMap<GridIndex, T> = HashMap::new();
        for item_type in set {
            self.grid.retain(|pos, item|{
                if *item == item_type {
                    true
                } else {
                    map.insert(*pos, *item);
                    false
                }
            })
        }
        self.save_system.submit_play(StackItem::BatchRemove(map));
    }

    pub fn clear_only(&mut self, set: HashSet<T>){
        let mut map: HashMap<GridIndex, T> = HashMap::new();
        for item_type in set {
            self.grid.retain(|pos, item|{
                if *item == item_type {
                    map.insert(*pos, *item);
                    false
                } else {
                    true
                }
            })
        }
        self.save_system.submit_play(StackItem::BatchRemove(map));
    }

}


//////////////////////////////////////////////////////////////////////////////////////////////////////
/// 
/// Grid Widget
/// 
/////////////////////////////////////////////////////////////////////////////////////////////////////

// TODO: Keep as widget to perform scaling/translation of child children paint methods
// TODO: Rename to CanvasWrapper
// TODO: Move Snapping System out of main to lib and attach to Canvas
// TODO: Replace grid with canvas
// TODO: Translate event position to correct location based on scaling/translation offset

#[derive(Clone, PartialEq, Data, Lens)]
pub struct GridWidget<T> {
    cell_size: f64,
    phantom: PhantomData<T>,
    start_pos: GridIndex,
    state: GridState,
    previous_stack_length: usize,
    previous_playback_index: usize,
}

impl<T> GridWidget<T> {
    pub fn new(cell_size: f64) -> Self {
        GridWidget {
            cell_size,
            phantom: PhantomData,
            start_pos: GridIndex { row: 0, col: 0 },
            state: GridState::Idle,
            previous_stack_length: 0,
            previous_playback_index: 0,
        }
    }

    fn grid_pos(&self, p: Point) -> Option<GridIndex> {
        if p.x < 0.0 || p.y < 0.0 || self.cell_size == 0.0 {
            return None;
        }
        let col = (p.x / self.cell_size) as usize;
        let row = (p.y / self.cell_size) as usize;

        Some(GridIndex { row, col })
    }

    pub fn invalidation_area(&self, pos: GridIndex) -> Rect {
        let point = Point {
            x: self.cell_size * pos.col as f64,
            y: self.cell_size * pos.row as f64,
        };
        Rect::from_origin_size(point, Size {width: self.cell_size, height: self.cell_size })
    }
}

impl<T:GridItem + PartialEq> Widget<GridWidgetData<T>> for GridWidget<T>{
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut GridWidgetData<T>, _env: &Env) {
        
        let mut change_tracker: HashSet<StackItem<T>> = HashSet::new();
        
        match &self.state{
            GridState::Idle => {
                // info!("Idle State");
                match event {
                    Event::Command(cmd) => {
                        if cmd.is(SET_DISABLED) {
                            self.state = GridState::Disabled;
                        } else if cmd.is(UPDATE_GRID_PLAYBACK) {
                            // info!("Playback index | {:?} vs {:?} | Stack Length", data.playback_index, data.save_stack.len());

                            let playback_diff = data.save_system.get_playback_index() as isize - self.previous_playback_index as isize;

                            let range = 1..=(playback_diff.abs() as usize);

                            for i in range {
                                if playback_diff > 0 {
                                    if let Some(item) = data.save_system.get(data.save_system.get_playback_index() - i) {
                                        item.forward(&mut data.grid);
                                        change_tracker.insert(item.clone());
                                    }
                                } else {
                                    if let Some(item) = data.save_system.get(data.save_system.get_playback_index() + i - 1) {
                                        item.reverse(&mut data.grid);
                                        change_tracker.insert(item.clone());
                                    }
                                }
                            }
                        } else if cmd.is(UPDATE_PAINT_PLAYBACK) {
                            let item = data.save_system.get(data.save_system.get_playback_index() - 1).unwrap();
                            change_tracker.insert(item.clone());
                        } else if cmd.is(REQUEST_PAINT) {
                            ctx.request_paint();
                        }
                    },
                    Event::MouseDown(e) => {
                        let grid_pos_opt = self.grid_pos(e.pos);
                            grid_pos_opt.iter().for_each(|pos| {
                                let option = data.grid.get(pos);
                                if self.state == GridState::Idle{
                                    if e.button == MouseButton::Left{
                                        // info!("Left Click");
                                        // info!("Start State: {:?}", self.state);
                                        // info!("Start Action: {:?}", data.action);
                                        match data.action {
                                            GridAction::Dynamic => {
                                                self.state = GridState::Running(GridAction::Dynamic);
                                                match option{
                                                    None => {
                                                        data.action = GridAction::Add;
                                                    },
                                                    Some(item)=> {
                                                        if *item == data.grid_item {
                                                            data.action = GridAction::Move;
                                                        } else {
                                                            data.action = GridAction::Add;                                
                                                        }
                                                    }
                                                }
                                            },
                                            GridAction::Move => {
                                                if option.is_some() {
                                                    self.state = GridState::Running(GridAction::Move);
                                                }
                                            },
                                            _ => {
                                                self.state = GridState::Running(data.action);
                                            },                                        
                                        }

                                    } else if e.button == MouseButton::Right{
                                        // info!("Right Click");
                                        if let GridAction::Dynamic = data.action{
                                            self.state = GridState::Running(data.action);
                                            data.action = GridAction::Remove;
                                        }
                                    }
                                }

                                if let GridState::Running(_) = self.state{
                                    if data.action == GridAction::Add {
                                        if data.add_node(pos, data.grid_item) {
                                            change_tracker.insert(data.save_system.get_last_item().unwrap().clone());
                                        }
                                    } else if data.action == GridAction::Remove && option.is_some(){
                                        if data.remove_node(pos) {
                                            change_tracker.insert(data.save_system.get_last_item().unwrap().clone());
                                        }
                                    } else if data.action == GridAction::Move && option.is_some() {
                                        self.start_pos = *pos;
                                    }
                                }
                            });
                        // info!("Acquire State: {:?}", self.state);
                        // info!("Acquire Action: {:?}", data.action);
                    },

                    _ => {},
                }
            },
            GridState::Running(_) => {
                // info!("Running State");
                match event {            
                    Event::MouseMove(e) => {
                        let grid_pos_opt = self.grid_pos(e.pos);
                        grid_pos_opt.iter().for_each(|pos| {
                            let option = data.grid.get(pos);
                            match data.action{
                                GridAction::Add => {
                                    if data.add_node(pos, data.grid_item) {
                                        change_tracker.insert(data.save_system.get_last_item().unwrap().clone());
                                    }                                    
                                },
                                GridAction::Move => {
                                    if self.start_pos != *pos {
                                        if data.move_node(&self.start_pos, pos) {
                                            change_tracker.insert(data.save_system.get_last_item().unwrap().clone());
                                            self.start_pos = *pos;
                                        }
                                    }
                                },
                                GridAction::Remove => {
                                    if option.is_some(){
                                        if data.remove_node(pos) {
                                            change_tracker.insert(data.save_system.get_last_item().unwrap().clone());
                                        }
                                    }        
                                },
                                _ => (),
                            }
                        });
                    },
        
                    Event::MouseUp(e) => {
                        if e.button == MouseButton::Right && self.state == GridState::Running(GridAction::Dynamic) && data.action == GridAction::Remove {
                            self.state = GridState::Idle;
                            data.action = GridAction::Dynamic;
                        } else if e.button == MouseButton::Left && self.state == GridState::Running(GridAction::Dynamic){
                            self.state = GridState::Idle;
                            data.action = GridAction::Dynamic;
                        } else if e.button == MouseButton::Left {
                            self.state = GridState::Idle;
                        }
                        // info!("Release State: {:?}", self.state);
                        // info!("Release Action: {:?}", data.action);
                    },
                    _ => {},
                }
            },
            GridState::Disabled => {
                if let Event::Command(cmd) = event {
                    if cmd.is(SET_ENABLED) {
                        self.state = GridState::Idle;
                    }
                }
            },
        }

        // To be removed with refactoring
        if change_tracker.len() != 0 {           
            for item in &change_tracker {
                for pos in item.get_positions().iter(){
                    ctx.request_paint_rect(self.invalidation_area(*pos));
                }
            }

            change_tracker.clear();
            self.previous_playback_index = data.save_system.get_playback_index();
        }
    }

    fn lifecycle(&mut self, _ctx: &mut LifeCycleCtx, _event: &LifeCycle, _data: &GridWidgetData<T>, _env: &Env) {
        // if let LifeCycle::WidgetAdded = event {
        //     self.previous_stack_length = data.save_stack.len();
        //     if data.playback_index != 0 {
        //         ctx.submit_command(UPDATE_GRID_PLAYBACK);
        //     }
        // }
    }

    fn update(
        &mut self,
        _ctx: &mut UpdateCtx,
        _old_data: &GridWidgetData<T>,
        _data: &GridWidgetData<T>,
        _env: &Env,
    ) {       
    }

    fn layout(
        &mut self,
        _ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        _data: &GridWidgetData<T>,
        _env: &Env,
    ) -> Size {
        let width = bc.max().width;
        let height = bc.max().height;
        //debug!("Box constraints width: {:?}", bc.max().width);
        //debug!("Box constraints height: {:?}", bc.max().height);

        Size {
            width: width,
            height: height,
        }
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &GridWidgetData<T>, _env: &Env) {
        //debug!("Running paint method");
        // Draw grid cells

        let screen_space = ctx.size();
        let damage_region = ctx.region().clone();

        // println!("Canvas Screen Space: {:?}", screen_space);
        // println!("Canvas Damage Region: {:?}\n", damage_region);
        let visible_columns = (screen_space.width / self.cell_size).ceil() as usize;
        let visible_rows =  (screen_space.height / self.cell_size).ceil() as usize;

        // Calculate area to render
        let paint_rectangles = damage_region.rects();

        for paint_rect in paint_rectangles.iter() {
            let from_grid_pos: GridIndex = self.grid_pos(paint_rect.origin()).unwrap();
            let from_row = from_grid_pos.row;
            let from_col = from_grid_pos.col;

            let to_grid_pos = self
                .grid_pos(Point::new(paint_rect.max_x(), paint_rect.max_y()))
                .unwrap_or(GridIndex {
                    col: visible_columns - 1,
                    row: visible_rows - 1,
                });
            let to_row = to_grid_pos.row;
            let to_col = to_grid_pos.col;

            //debug!("Bounding box with origin {:?} and dimensions {:?} × {:?}", paint_rect.origin(), paint_rect.width(), paint_rect.height());
            //debug!("Paint from row: {:?} to row {:?}", from_row, to_row);
            //debug!("Paint from col: {:?} to col {:?}", from_col, to_col);

            // Partial Area Paint Logic

            for row in from_row..=to_row {
                for col in from_col..=to_col {
                    let point = Point {
                        x: self.cell_size * col as f64,
                        y: self.cell_size * row as f64,
                    };
                    let rect = Rect::from_origin_size(point, Size {width: self.cell_size, height: self.cell_size});

                    let grid_pos = GridIndex { row, col };

                    if let Some(runner) = data.grid.get(&grid_pos){
                        ctx.fill(rect, runner.get_color());
                    }
                }
            }
        }
    }
}

////////////////////////////////////////////////////////////////////////////////////////////
///
/// CanvasWrapper
/// 
////////////////////////////////////////////////////////////////////////////////////////////

pub struct CanvasWrapper<T, W>{
    child: WidgetPod<T, W>,
}

impl<T, W:Widget<T>> CanvasWrapper<T, W> {
    pub fn new(inner: W) -> Self {
        CanvasWrapper { child: WidgetPod::new(inner) }
    }
}

impl<T: Data + PanningData + ZoomData, W: Widget<T>> Widget<T> for CanvasWrapper<T, W> {
    fn event(&mut self, ctx: &mut druid::EventCtx, event: &Event, data: &mut T, env: &druid::Env) {
        self.child.event(ctx, event, data, env);
    }

    fn lifecycle(&mut self, ctx: &mut druid::LifeCycleCtx, event: &druid::LifeCycle, data: &T, env: &druid::Env) {
        self.child.lifecycle(ctx, event, data, env);
    }

    fn update(&mut self, ctx: &mut druid::UpdateCtx, _old_data: &T, data: &T, env: &druid::Env) {
        self.child.update(ctx, data, env);
    }

    fn layout(&mut self, ctx: &mut druid::LayoutCtx, bc: &druid::BoxConstraints, data: &T, env: &druid::Env) -> Size {
        let size = self.child.layout(ctx, bc, data, env);
        self.child.set_origin(ctx, data, env, Point::ORIGIN);
        size
    }

    fn paint(&mut self, ctx: &mut druid::PaintCtx, data: &T, env: &druid::Env) {
        
        let mut pos = Point::new(50.0, 50.0);

        // pos.x += data.get_translation_offset().x;
        // pos.y += data.get_translation_offset().y;

        ctx.with_save(|ctx| {
            let translate = Affine::translate(data.get_absolute_offset().to_vec2());
            let scale = Affine::scale(data.get_zoom_scale());
            ctx.transform(translate);
            ctx.transform(scale);
            ctx.fill(Circle::new(pos, 10.0), &white::PLATINUM);
            self.child.paint(ctx, data, env);
        });

        
        // println!("Offset: {:?}", data.get_translation_offset());
        
    }
}