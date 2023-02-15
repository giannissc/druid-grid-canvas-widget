///////////////////////////////////////////////////////////////////////////////////////////////////
/// 
/// Imports
/// 
///////////////////////////////////////////////////////////////////////////////////////////////////
use std::fmt::Debug;
use druid::{im::{HashMap, HashSet, Vector}, Data, Rect, Point, Size, Widget, EventCtx, Event, Env, 
Selector, MouseButton, LifeCycleCtx, LifeCycle, UpdateCtx, LayoutCtx, BoxConstraints, PaintCtx, 
Affine, RenderContext, Lens, widget::{Label, LabelText, Button}, Insets, Color, WidgetPod};
use druid_color_thesaurus::white;

use crate::{GridItem, snapping::GridSnapData, save_system::SaveSystemData, StackItem, GridAction, GridState, GridIndex, canvas::{Canvas, PointKey,},};


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

//////////////////////////////////////////////////////////////////////////////////////
//
// GridWidgetData
//
//////////////////////////////////////////////////////////////////////////////////////
#[derive(Clone, Data, Lens, PartialEq, Debug)]
pub struct GridCanvasData<T: GridItem + PartialEq + Debug>{
    pub action: GridAction,
    pub grid_item: T,
    pub grid: HashMap<GridIndex, T>,
    // Data Hierarchy
    pub save_data: SaveSystemData<StackItem<T>>,
    pub snap_data: GridSnapData,
    pub position: Point, 
}

impl<T: GridItem + PartialEq + Debug> GridCanvasData<T> {
    pub fn new(item_type: T) -> Self {
        Self {
            action: GridAction::Dynamic,
            grid_item: item_type,
            grid: HashMap::new(),
            save_data: SaveSystemData::new(),
            snap_data: GridSnapData::new(50.0),
            position: Point::ZERO,
        }
    }

    pub fn with_cell_size(&mut self, cell_size: f64) {
        self.snap_data.cell_size = cell_size;
    }

    // Basic Grid methods
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
            self.save_data.submit_and_process(command_item);
            return true;
        }
        false
    }

    fn remove_node(&mut self, pos: &GridIndex) -> bool {
        if let Some(item) = self.grid.remove(pos){
            if item.can_remove(){
                let command_item = StackItem::Remove(*pos, item);
                self.save_data.submit_and_process(command_item);
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
            self.save_data.submit_and_process(command_item);
            return true;
        }
        false
    }
    
    // Auxiliary Grid Methods
    pub fn add_node_perimeter(
        &mut self,
        pos: GridIndex,
        row_n: isize,
        column_n: isize,
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
        self.save_data.submit_and_process(StackItem::BatchAdd(map))

    }

    // Clear Grid methods
    pub fn clear_all(&mut self){
        self.save_data.submit_and_process(StackItem::BatchRemove(self.grid.clone()));
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
        self.save_data.submit_and_process(StackItem::BatchRemove(map));
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
        self.save_data.submit_and_process(StackItem::BatchRemove(map));
    }

    // Save stack methods
    fn validate_stack_list(&mut self, list: Vector<StackItem<T>>) -> (HashMap<GridIndex, T>, Vector<StackItem<T>>){
        let mut stack_list = Vector::new();
        let mut pos_map = HashMap::new();
        
        for stack_item in list {
            match stack_item {
                StackItem::Add(pos, current_item, _) => {
                    let option = self.grid.get(&pos);
                    if current_item.can_add(option) {
                        stack_list.push_back(stack_item);
                        pos_map.insert(pos, current_item);
                    }
                },
                StackItem::BatchAdd(mut map) => {
                    map.retain(|pos, (current_item, _)|{
                        let option = self.grid.get(pos);
                        if current_item.can_add(option) {
                            pos_map.insert(*pos, *current_item);
                        }
                        current_item.can_add(option)
                    });

                    if !map.is_empty(){
                        stack_list.push_back(StackItem::BatchAdd(map));
                    }
                },
                _ => (),
            }
        }
        (pos_map, stack_list)
    }

    pub fn submit_to_stack(&mut self, list: Vector<StackItem<T>>){
        let (_, save_list) = self.validate_stack_list(list);
        self.save_data.append(save_list);

    }

    pub fn submit_to_stack_and_process(&mut self, list: Vector<StackItem<T>>){
        let (pos_map, save_list) = self.validate_stack_list(list);
        for (pos, item) in pos_map.iter(){
            self.grid.insert(*pos, *item);
        }
        self.save_data.append_and_process(save_list);
    }
}

//////////////////////////////////////////////////////////////////////////////////////////////////////
/// 
/// GridCanvas Widget
/// 
/////////////////////////////////////////////////////////////////////////////////////////////////////

// TODO: Keep as widget to perform scaling/translation of child children paint methods
// TODO: Move Snapping System out of main to lib and attach to Canvas
// TODO: Add canvas to grid widget
pub struct GridCanvas<T: GridItem + PartialEq + Debug> where GridCanvasData<T>: Data {
    start_pos: GridIndex,
    state: GridState,
    // previous_stack_length: usize,
    previous_playback_index: usize,
    previous_direction: isize,
    // canvas: WidgetPod<GridCanvasData<T>, Canvas<GridCanvasData<T>>>,
    canvas: Canvas<GridCanvasData<T>>,

}

impl<T: Clone + GridItem + Debug> GridCanvas<T> where GridCanvasData<T>: Data {
    pub fn new() -> Self {
        let mut canvas = Canvas::new();
        GridCanvas {
            start_pos: GridIndex { row: 0, col: 0 },
            state: GridState::Idle,
            // previous_stack_length: 0,
            previous_playback_index: 0,
            previous_direction: 0,
            // canvas: WidgetPod::new(canvas),
            canvas,
        }
    }

    pub fn invalidation_area(&self, pos: GridIndex, cell_size: f64) -> Rect {
        let point = Point {
            x: cell_size * pos.col as f64,
            y: cell_size * pos.row as f64,
        };
        Rect::from_origin_size(point, Size {width: cell_size, height: cell_size })
    }
}

impl<T:GridItem + PartialEq + Debug> Widget<GridCanvasData<T>> for GridCanvas<T> where GridCanvasData<T>: Data {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut GridCanvasData<T>, env: &Env) {
        // println!("Canvas Wrapper Event");
        
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
                            let playback_diff = data.save_data.playback_index as isize - self.previous_playback_index as isize;

                            let range = 1..=(playback_diff.abs() as usize);

                            for i in range {
                                let foo = data.clone();
                                if playback_diff > 0 {
                                    if let Some(item) = data.save_data.save_stack.get(data.save_data.playback_index - i) {
                                        item.forward(&mut data.grid, &mut self.canvas, foo);
                                        change_tracker.insert(item.clone());
                                        ctx.children_changed();
                                    }
                                } else {
                                    if let Some(item) = data.save_data.save_stack.get(data.save_data.playback_index + i - 1) {
                                        item.reverse(&mut data.grid, &mut self.canvas, foo);
                                        change_tracker.insert(item.clone());
                                        ctx.children_changed();
                                    }
                                }
                            }
                        } else if cmd.is(UPDATE_PAINT_PLAYBACK) {
                            let item = data.save_data.save_stack.get(data.save_data.playback_index - 1).unwrap();
                            change_tracker.insert(item.clone());
                        } else if cmd.is(REQUEST_PAINT) {
                            ctx.request_paint();
                        }
                    },
                    Event::MouseDown(e) => {
                        let (row, col) = data.snap_data.get_grid_index(e.pos);
                        let grid_index = GridIndex::new(row, col);
                        let option = data.grid.get(&grid_index);

                        let position = data.snap_data.get_grid_position(grid_index.row, grid_index.col);
                        let size = Size::new(data.snap_data.cell_size, data.snap_data.cell_size);

                        // println!("Mouse Point: {:?}", e.pos);
                        // println!("Grid Index: {:?}", grid_index);
                        // println!("Grid Point: {:?}", position);

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
                                                    data.action = GridAction::Move
                                                } else {
                                                    data.action = GridAction::Add                            
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
                                if let GridAction::Dynamic = data.action {
                                    self.state = GridState::Running(data.action);
                                    data.action = GridAction::Remove;
                                }
                            }
                        }

                        if let GridState::Running(_) = self.state{
                            if data.action == GridAction::Add {
                                if data.add_node(&grid_index, data.grid_item) {
                                    // let foo = self.canvas.add_child(ctx, child, to);
                                    change_tracker.insert(data.save_data.save_stack.last().unwrap().clone());
                                    
                                    let child = GridChild::new(data.grid_item.get_short_text(), data.grid_item.get_color(), size);
                                    
                                    // self.canvas.widget_mut().add_child(child, position.into());
                                    self.canvas.add_child(child, position.into());
                                    ctx.children_changed();
                                }
                            } else if data.action == GridAction::Remove && option.is_some(){
                                if data.remove_node(&grid_index) {
                                    change_tracker.insert(data.save_data.save_stack.last().unwrap().clone());
                                    // self.canvas.widget_mut().remove_child(position.into());
                                    self.canvas.remove_child(position.into());
                                    ctx.children_changed();
                                }
                            } else if data.action == GridAction::Move && option.is_some() {
                                self.start_pos = grid_index;
                            }
                        }
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
                        let (row, col) = data.snap_data.get_grid_index(e.pos);
                        let grid_index = GridIndex::new(row, col);
                        let option = data.grid.get(&grid_index);

                        let position = data.snap_data.get_grid_position(grid_index.row, grid_index.col);
                        let size = Size::new(data.snap_data.cell_size, data.snap_data.cell_size);

                        // println!("Mouse Point: {:?}", e.pos);
                        // println!("Grid Index: {:?}", grid_index);
                        // println!("Grid Point: {:?}", position);

                        match data.action{
                            GridAction::Add => {
                                if data.add_node(&grid_index, data.grid_item) {
                                    change_tracker.insert(data.save_data.save_stack.last().unwrap().clone());
                                    let child = GridChild::new(data.grid_item.get_short_text(), data.grid_item.get_color(), size);
                                    // self.canvas.widget_mut().add_child(child, position.into());
                                    self.canvas.add_child(child, position.into());
                                    ctx.children_changed();
                                }                                    
                            },
                            GridAction::Move => {
                                if self.start_pos != grid_index {
                                    if data.move_node(&self.start_pos, &grid_index) {
                                        change_tracker.insert(data.save_data.save_stack.last().unwrap().clone());
                                        // let from = data.snap_data.get_grid_position(self.start_pos.row, self.start_pos.row).into();
                                        // let to = data.snap_data.get_grid_position(grid_index.row, grid_index.row).into();
                                        // println!("{:?} -> {:?}", self.start_pos, grid_index);
                                        // println!("{:?} -> {:?}", from, to);
                                        // self.canvas.move_child(ctx, from, to);
                                        self.start_pos = grid_index;
                                    }
                                }
                            },
                            GridAction::Remove => {
                                if option.is_some(){
                                    if data.remove_node(&grid_index) {
                                        change_tracker.insert(data.save_data.save_stack.last().unwrap().clone());
                                        // self.canvas.widget_mut().remove_child(position.into());
                                        self.canvas.remove_child(position.into());
                                        ctx.children_changed();
                                    }
                                }        
                            },
                            _ => (),
                        }
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
                    ctx.request_paint_rect(self.invalidation_area(*pos, data.snap_data.cell_size));
                }
            }

            change_tracker.clear();
            ctx.request_update();
            self.previous_playback_index = data.save_data.playback_index;
        }

        self.canvas.event(ctx, event, data, env);
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &GridCanvasData<T>, env: &Env) {
        // println!("Canvas Wrapper ({:?}) Lifecycle: {:?}", ctx.widget_id(), event);

        // if let LifeCycle::Internal(RouteWidgetAdded) = event {
        //     self.canvas.lifecycle(ctx, &LifeCycle::WidgetAdded, data, env);
        // }

        self.canvas.lifecycle(ctx, event, data, env);
    }

    fn update(&mut self, ctx: &mut UpdateCtx, old_data: &GridCanvasData<T>, data: &GridCanvasData<T>, env: &Env){
        // println!("Canvas Wrapper Update");
        // self.canvas.update(ctx, data, env);
        println!("=====================================");
        println!("Old Grid | {:?} vs {:?} | New Grid", old_data.grid.len(), data.grid.len());
        println!("Old Save | {:?} vs {:?} | New Save", old_data.save_data.save_stack.len(), data.save_data.save_stack.len());
        println!("Old Play | {:?} vs {:?} | New Play", self.previous_playback_index, data.save_data.playback_index);
        println!("Canvas Children: {:?}\n", self.canvas.children_len());

        let index = data.save_data.playback_index;
        let current_direction = index as isize - self.previous_playback_index as isize;
        
        let offset = 0;
        
        while data.grid.len() != self.canvas.children_len() {
            if self.previous_direction > 0 { //redo case
                // if grid > canvas action is an addition -> just read type and location and add to canvas (need separate case for batch)
                // if grid < canvas action is a deletion -> just read location and delete from canvas (need separate case for batch)
                println!("Positive direction");
                let foo = data.save_data.save_stack.get(index+offset);
                println!("Save Item: {:?}", foo);
                break;

            } else if self.previous_direction < 0 { //undo case
                println!("Negative direction");
                break;

            } else { // submit_and_process case
                println!("No direction");
                break;

            }
        }

        self.previous_direction = current_direction;
        
        self.canvas.update(ctx, old_data, data, env);
        }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, data: &GridCanvasData<T>, env: &Env) -> Size {
        // println!("Canvas Wrapper Layout");
        let origin = Point::new(0., 0.);
        //debug!("Box constraints width: {:?}", bc.max().width);
        //debug!("Box constraints height: {:?}", bc.max().height);
        self.canvas.layout(ctx, bc, data, env);
        // self.canvas.set_origin(ctx, data, env, origin);


        let width = bc.max().width;
        let height = bc.max().height;

        Size {
            width: width,
            height: height,
        }
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &GridCanvasData<T>, env: &Env) {
        // println!("Canvas Wrapper Paint");
        //debug!("Running paint method");
        // Draw grid cells
        let damage_region = ctx.region().clone();

        // Calculate area to render
        let paint_rectangles = damage_region.rects();

        let size = ctx.size();

        ctx.with_save(|ctx| {
            let translate = Affine::translate(data.snap_data.pan_data.absolute_offset.to_vec2());
            let scale = Affine::scale(data.snap_data.zoom_data.zoom_scale);
            
            ctx.transform(translate);
            ctx.transform(scale);

            // self.canvas.paint_always(ctx, data, env);
            self.canvas.paint(ctx, data, env);
        });
    }
}
///////////////////////////////////////////////////////////////////////////////////////////////////

const LABEL_INSETS: Insets = Insets::uniform_xy(1., 1.);

pub struct GridChild<T> {
    label_text: Label<T>,
    label_size: Size, // Needed to shift label to correct position when painting
    color: Color,
    size: Size,
}

impl<T:Data> GridChild<T> {
    pub fn new(text: impl Into<LabelText<T>>, color: Color, size: Size) -> Self {
        // let foo = Label::new(tooltip_text).tooltip();
        let mut label_text = Label::new(text);
        label_text.set_text_color(white::ALABASTER);
        label_text.set_text_size(16.);

        GridChild {
            label_text,
            label_size: Size::ZERO,
            color,
            size,
        }
    }
}

impl<T:Data> Widget<T> for GridChild<T> {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env) {
        // Add tooltip logic on hover
        
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &T, env: &Env) {
        // println!("Canvas Child Lifecycle: {:?}", event);

        if let LifeCycle::HotChanged(_) | LifeCycle::DisabledChanged(_) = event {
            ctx.request_paint();
        }

        // if let LifeCycle::Internal(RouteWidgetAdded) = event {
        //     self.label_text.lifecycle(ctx, &LifeCycle::WidgetAdded, data, env);
        // }

        self.label_text.lifecycle(ctx, event, data, env);
    }

    fn update(&mut self, ctx: &mut UpdateCtx, old_data: &T, data: &T, env: &Env) {
        self.label_text.update(ctx, old_data, data, env);
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, data: &T, env: &Env) -> Size {
        let padding = Size::new(LABEL_INSETS.x_value(), LABEL_INSETS.y_value());
        let label_bc = bc.shrink(padding).loosen();
        self.label_size = self.label_text.layout(ctx, &label_bc, data, env);
        let baseline = self.label_text.baseline_offset();
        ctx.set_baseline_offset(baseline + LABEL_INSETS.y1);
        let actual_size = bc.constrain(self.size);
        actual_size
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &T, env: &Env) {
        let is_active = ctx.is_active() && !ctx.is_disabled();
        let is_hot = ctx.is_hot();
        let size = ctx.size();
        // A hack to get it to do the right thing
        // let rect = Rect::from_origin_size(self.position, self.size);
        let rect = size.to_rect();

        ctx.fill(rect, &self.color);

        let label_offset = (size.to_vec2() - self.label_size.to_vec2()) / 2.0;

        ctx.with_save(|ctx| {
            ctx.transform(Affine::translate(label_offset));
            self.label_text.paint(ctx, data, env);
        });
    }
}