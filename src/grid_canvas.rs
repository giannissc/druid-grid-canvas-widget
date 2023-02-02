///////////////////////////////////////////////////////////////////////////////////////////////////
/// 
/// Imports
/// 
///////////////////////////////////////////////////////////////////////////////////////////////////
use std::{marker::PhantomData};

use druid::{im::{HashMap, HashSet, Vector}, Data, Rect, Point, Size, Widget, EventCtx, Event, Env, Selector, MouseButton, LifeCycleCtx, LifeCycle, UpdateCtx, LayoutCtx, BoxConstraints, PaintCtx, Affine, RenderContext, Lens};

use crate::{GridItem, snapping::GridSnapData, save_system::SaveSystemData, StackItem, GridAction, GridState, GridIndex, canvas::Canvas,};

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
#[derive(Clone, Data, Lens, PartialEq)]
pub struct GridCanvasData<T: GridItem + PartialEq> {
    pub action: GridAction,
    pub grid_item: T,
    pub grid: HashMap<GridIndex, T>,
    // Data Hierarchy
    pub save_data: SaveSystemData<StackItem<T>>,
    pub snap_data: GridSnapData,
}

impl<T: GridItem + PartialEq> GridCanvasData<T> {
    pub fn new(item_type: T, cell_size: f64) -> Self {
        Self {
            action: GridAction::Dynamic,
            grid_item: item_type,
            grid: HashMap::new(),
            save_data: SaveSystemData::new(),
            snap_data: GridSnapData::new(cell_size)
        }
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
    fn add_node_perimeter(
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
    fn clear_all(&mut self){
        self.save_data.submit_and_process(StackItem::BatchRemove(self.grid.clone()));
        self.grid.clear();
    }
    fn clear_except(&mut self, set: HashSet<T>){
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
    fn clear_only(&mut self, set: HashSet<T>){
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
pub struct GridCanvas<T, U> {
    start_pos: GridIndex,
    state: GridState,
    // previous_stack_length: usize,
    previous_playback_index: usize,
    canvas: Canvas<U>,
    phantom: PhantomData<T>,
}

impl<T: Clone + GridItem, U:Data> GridCanvas<T, U> {
    pub fn new() -> Self {
        GridCanvas {
            start_pos: GridIndex { row: 0, col: 0 },
            state: GridState::Idle,
            // previous_stack_length: 0,
            previous_playback_index: 0,
            canvas: Canvas::new(),
            phantom: PhantomData,
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

impl<T:GridItem + PartialEq, U: Data> Widget<GridCanvasData<T>> for GridCanvas<T, U> {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut GridCanvasData<T>, _env: &Env) {
        
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
                                
                                if playback_diff > 0 {
                                    if let Some(item) = data.save_data.save_stack.get(data.save_data.playback_index - i) {
                                        item.forward(&mut data.grid);
                                        change_tracker.insert(item.clone());
                                    }
                                } else {
                                    if let Some(item) = data.save_data.save_stack.get(data.save_data.playback_index + i - 1) {
                                        item.reverse(&mut data.grid);
                                        change_tracker.insert(item.clone());
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
                        let pos = GridIndex::new(row, col);
                        let option = data.grid.get(&pos);
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
                                if data.add_node(&pos, data.grid_item) {
                                    // let foo = self.canvas.add_child(ctx, child, to);
                                    change_tracker.insert(data.save_data.save_stack.last().unwrap().clone());
                                }
                            } else if data.action == GridAction::Remove && option.is_some(){
                                if data.remove_node(&pos) {
                                    change_tracker.insert(data.save_data.save_stack.last().unwrap().clone());
                                }
                            } else if data.action == GridAction::Move && option.is_some() {
                                self.start_pos = pos;
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
                        let pos = GridIndex::new(row, col);
                        let option = data.grid.get(&pos);
                        match data.action{
                            GridAction::Add => {
                                if data.add_node(&pos, data.grid_item) {
                                    change_tracker.insert(data.save_data.save_stack.last().unwrap().clone());
                                }                                    
                            },
                            GridAction::Move => {
                                if self.start_pos != pos {
                                    if data.move_node(&self.start_pos, &pos) {
                                        change_tracker.insert(data.save_data.save_stack.last().unwrap().clone());
                                        self.start_pos = pos;
                                    }
                                }
                            },
                            GridAction::Remove => {
                                if option.is_some(){
                                    if data.remove_node(&pos) {
                                        change_tracker.insert(data.save_data.save_stack.last().unwrap().clone());
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

        // self.canvas.event(ctx, event, data, env);

        // To be removed with refactoring
        if change_tracker.len() != 0 {           
            for item in &change_tracker {
                for pos in item.get_positions().iter(){
                    ctx.request_paint_rect(self.invalidation_area(*pos, data.snap_data.cell_size));
                }
            }

            change_tracker.clear();
            self.previous_playback_index = data.save_data.playback_index;
        }
    }

    fn lifecycle(&mut self, _ctx: &mut LifeCycleCtx, _event: &LifeCycle, _data: &GridCanvasData<T>, _env: &Env) {
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
        _old_data: &GridCanvasData<T>,
        _data: &GridCanvasData<T>,
        _env: &Env,
    ) {       
    }

    fn layout(
        &mut self,
        _ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        _data: &GridCanvasData<T>,
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

    fn paint(&mut self, ctx: &mut PaintCtx, data: &GridCanvasData<T>, _env: &Env) {
        //debug!("Running paint method");
        // Draw grid cells
        let damage_region = ctx.region().clone();

        // println!("Canvas Screen Space: {:?}", screen_space);
        // println!("Canvas Damage Region: {:?}\n", damage_region);

        // Calculate area to render
        let paint_rectangles = damage_region.rects();

        for paint_rect in paint_rectangles.iter() {
            let (row, col) =  data.snap_data.get_grid_index(paint_rect.origin());
            let from_grid_pos: GridIndex = GridIndex::new(row, col);
            let from_row = from_grid_pos.row;
            let from_col = from_grid_pos.col;

            let (row, col) =  data.snap_data.get_grid_index(Point::new(paint_rect.max_x(), paint_rect.max_y()));
            let to_grid_pos = GridIndex::new(row, col);
            let to_row = to_grid_pos.row;
            let to_col = to_grid_pos.col;

            //debug!("Bounding box with origin {:?} and dimensions {:?} × {:?}", paint_rect.origin(), paint_rect.width(), paint_rect.height());
            //debug!("Paint from row: {:?} to row {:?}", from_row, to_row);
            //debug!("Paint from col: {:?} to col {:?}", from_col, to_col);

            // Partial Area Paint Logic
            ctx.with_save(|ctx| {
                let translate = Affine::translate(data.snap_data.pan_data.absolute_offset.to_vec2());
                let scale = Affine::scale(data.snap_data.zoom_data.zoom_scale);
                ctx.transform(translate);
                ctx.transform(scale);

                for row in from_row..=to_row {
                    for col in from_col..=to_col {
                        let grid_pos = GridIndex { row, col };
                        let rect = self.invalidation_area(grid_pos, data.snap_data.cell_size);

                        if let Some(item) = data.grid.get(&grid_pos){
                            ctx.fill(rect, item.get_color());
                        }
                    }
                }
            });
        }
    }
}