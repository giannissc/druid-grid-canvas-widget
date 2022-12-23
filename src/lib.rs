use std::hash::Hash;
use std::marker::PhantomData;
use druid::{BoxConstraints, Data, Env, Event, EventCtx, LayoutCtx, Lens, LifeCycle,
    LifeCycleCtx, PaintCtx, RenderContext, UpdateCtx, Widget, Selector, Point, Rect, Size, Color, MouseButton,};

use druid::im::{HashMap, Vector, HashSet};

use druid_color_thesaurus::*;
use log::info;

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
/// GridNodePosition
/// 
/////////////////////////////////////////////////////////////////////////////////////////////////////
#[derive(Clone, Data, Copy, PartialEq, Debug, Hash, Eq)]
pub struct GridNodePosition {
    pub row: usize,
    pub col: usize,
}

impl GridNodePosition {
    pub fn above(self) -> GridNodePosition {
        GridNodePosition {
            row: self.row - 1,
            col: self.col,
        }
    }

    pub fn below(self) -> GridNodePosition {
        GridNodePosition {
            row: self.row + 1,
            col: self.col,
        }
    }

    pub fn left(self) -> GridNodePosition {
        GridNodePosition {
            row: self.row,
            col: self.col - 1,
        }
    }

    pub fn right(self) -> GridNodePosition {
        GridNodePosition {
            row: self.row,
            col: self.col + 1,
        }
    }

    // Also known in vlsi as the Manhattan Architecture
    pub fn neighbors_rectilinear(self) -> [GridNodePosition; 4] {
        let above = self.above();
        let below = self.below();
        let left = self.left();
        let right = self.right();
        [above, below, left, right]
    }

    // Also known in vlsi as the X Architecture
    pub fn neighbors_diagonal(self) -> [GridNodePosition; 4] {
        let above = self.above();
        let below = self.below();
        let left = self.left();
        let right = self.right();
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
pub trait GridRunner: Copy + Clone + Hash + Eq{
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
    Panning,
}

//////////////////////////////////////////////////////////////////////////////////////
//
// StackItem
//
//////////////////////////////////////////////////////////////////////////////////////
#[derive(Clone, PartialEq, Data, Debug, Hash, Eq)]
pub enum StackItem<T: GridRunner>{
    Add(GridNodePosition, T, Option<T>),
    Remove(GridNodePosition, T),
    Move(GridNodePosition, GridNodePosition, T),
    BatchAdd(HashMap<GridNodePosition, (T, Option<T>)>),
    BatchRemove(HashMap<GridNodePosition, T>),
}

impl<T: GridRunner> StackItem<T>{
    fn get_positions(&self) -> HashSet<GridNodePosition>{
        let mut set: HashSet<GridNodePosition> = HashSet::new();
        
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

    fn forward(&self, grid: &mut HashMap<GridNodePosition, T>){
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

    fn reverse(&self, grid: &mut HashMap<GridNodePosition, T>){
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
pub struct GridWidgetData<T:GridRunner + PartialEq>{
    pub grid: HashMap<GridNodePosition, T>,
    save_stack: Vector<StackItem<T>>,
    pub show_grid_axis: bool,
    pub action: GridAction,
    pub node_type: T,
    pub playback_index: usize,
}

impl<T:GridRunner + PartialEq> GridWidgetData<T>{
    pub fn new(initial_node: T) -> Self {
        GridWidgetData {
            grid: HashMap::new(),
            save_stack: Vector::new(),
            show_grid_axis: true,
            action: GridAction::Dynamic,
            node_type: initial_node,
            playback_index: 0,
        }
    }

    fn add_node(&mut self, pos: &GridNodePosition, item: T) -> bool{
        let option = self.grid.get(pos);

        let command_item;
        if option.is_none() {
            command_item = StackItem::Add(*pos, item, None);
        } else {
            command_item = StackItem::Add(*pos, item, Some(*option.unwrap()));
        }
        
        if item.can_add(option){
            self.grid.insert(*pos, item);
            self.save_stack.push_back(command_item);
            return true;
        }
        false
    }

    fn remove_node(&mut self, pos: &GridNodePosition) -> bool{
        if let Some(item) = self.grid.remove(pos){
            if item.can_remove(){
                let command_item = StackItem::Remove(*pos, item);
                self.save_stack.push_back(command_item);
                return true;
            } else {
                self.grid.insert(*pos, item);
            }
        }
        false
    }

    fn move_node(&mut self, from: &GridNodePosition, to:&GridNodePosition) -> bool{
        let item = self.grid.get(from).unwrap();
        let other = self.grid.get(to);
        if item.can_move(other) {
            let item = self.grid.remove(from).unwrap();
            self.grid.insert(*to, item);
            let command_item = StackItem::Move(*from, *to, item);
            self.save_stack.push_back(command_item);
            return true;
        }
        false
    }

    pub fn add_node_perimeter(
        &mut self,
        pos: GridNodePosition,
        row_n: usize,
        column_n: usize,
        tool: T,
    ) {
        let mut map: HashMap<GridNodePosition, (T, Option<T>)> = HashMap::new();
        for row in pos.row..pos.row + row_n {
            //debug!("Add node perimeter");
            //debug!("Row: {:?}", row);
            if row == pos.row || row == pos.row + row_n - 1 {
                // Top and Bottom Boundaries
                //debug!("Printing top/bottom boundary");
                for column in pos.col..pos.col + column_n {
                    map.insert(
                        GridNodePosition {
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
                    GridNodePosition {
                        row: row,
                        col: pos.col,
                    },
                    (tool, None),
                );
                // Right Boundary
                map.insert(
                    GridNodePosition {
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

        self.save_stack.push_back(StackItem::BatchAdd(map));

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

        self.save_stack.append(val_list)
    }

    pub fn get_stack_length(&self) -> usize {
        self.save_stack.len()
    }

    pub fn get_grid(&self) -> &HashMap<GridNodePosition, T> {
        &self.grid
    }

    pub fn clear_all(&mut self){
        self.save_stack.push_back(StackItem::BatchRemove(self.grid.clone()));
        self.grid.clear();
    }

    pub fn clear_except(&mut self, set: HashSet<T>){
        let mut map: HashMap<GridNodePosition, T> = HashMap::new();
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
        self.save_stack.push_back(StackItem::BatchRemove(map));
    }

    pub fn clear_only(&mut self, set: HashSet<T>){
        let mut map: HashMap<GridNodePosition, T> = HashMap::new();
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
        self.save_stack.push_back(StackItem::BatchRemove(map));
    }

}


//////////////////////////////////////////////////////////////////////////////////////////////////////
/// 
/// Grid Widget
/// 
/////////////////////////////////////////////////////////////////////////////////////////////////////


#[derive(Clone, PartialEq, Data, Lens)]
pub struct GridWidget<T> {
    max_rows: usize,
    max_columns: usize,
    min_cell_size: Size,
    visible_rows: usize,
    visible_columns: usize,
    chosen_cell_size: Size,
    left_corner_point: GridNodePosition,
    phantom: PhantomData<T>,
    start_pos: GridNodePosition,
    state: GridState,
    previous_stack_length: usize,
    previous_playback_index: usize,
}

impl<T> GridWidget<T> {
    pub fn new(rows: usize, columns: usize, cell_size: Size) -> Self {
        GridWidget {
            max_rows: rows,
            max_columns: columns,
            min_cell_size: cell_size,
            visible_columns: columns,
            visible_rows: rows,
            chosen_cell_size: Size {
                width: 0.0,
                height: 0.0,
            },
            left_corner_point: GridNodePosition { row: 0, col: 0 },
            phantom: PhantomData,
            start_pos: GridNodePosition { row: 0, col: 0 },
            state: GridState::Idle,
            previous_stack_length: 0,
            previous_playback_index: 0,
        }
    }

    fn grid_pos(&self, p: Point) -> Option<GridNodePosition> {
        let w0 = self.chosen_cell_size.width;
        let h0 = self.chosen_cell_size.height;
        if p.x < 0.0 || p.y < 0.0 || w0 == 0.0 || h0 == 0.0 {
            return None;
        }
        let col = (p.x / w0) as usize;
        let row = (p.y / h0) as usize;
        if col >= self.max_columns || row >= self.max_rows {
            return None;
        }
        Some(GridNodePosition { row, col })
    }

    pub fn invalidation_area(&self, pos: GridNodePosition) -> Rect {
        let point = Point {
            x: self.chosen_cell_size.width * pos.col as f64,
            y: self.chosen_cell_size.height * pos.row as f64,
        };
        Rect::from_origin_size(point, self.chosen_cell_size)
    }
}

impl<T:GridRunner + PartialEq> Widget<GridWidgetData<T>> for GridWidget<T>{
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
                            info!("Playback index | {:?} vs {:?} | Stack Length", data.playback_index, data.save_stack.len());

                            let playback_diff = data.playback_index as isize - self.previous_playback_index as isize;

                            let range = 1..=(playback_diff.abs() as usize);

                            for i in range {
                                if playback_diff > 0 {
                                    if let Some(item) = data.save_stack.get(data.playback_index-i) {
                                        item.forward(&mut data.grid);
                                        change_tracker.insert(item.clone());
                                    }
                                } else {
                                    if let Some(item) = data.save_stack.get(data.playback_index+i-1) {
                                        item.reverse(&mut data.grid);
                                        change_tracker.insert(item.clone());
                                    }
                                }
                            }
                        } else if cmd.is(UPDATE_PAINT_PLAYBACK) {
                            let item = data.save_stack.get(data.playback_index - 1).unwrap();
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
                                        info!("Left Click");
                                        info!("Start State: {:?}", self.state);
                                        info!("Start Action: {:?}", data.action);
                                        match data.action {
                                            GridAction::Dynamic => {
                                                self.state = GridState::Running(GridAction::Dynamic);
                                                match option{
                                                    None => {
                                                        data.action = GridAction::Add;
                                                    },
                                                    Some(item)=> {
                                                        if *item == data.node_type {
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
                                        info!("Right Click");
                                        if let GridAction::Dynamic = data.action{
                                            self.state = GridState::Running(data.action);
                                            data.action = GridAction::Remove;
                                        }
                                    } else if e.button == MouseButton::Middle{
                                        info!("Middle Click");
                                        if let GridAction::Dynamic = data.action{
                                            self.state = GridState::Running(data.action);
                                                data.action = GridAction::Panning;
                                        }
                                    }
                                }

                                if let GridState::Running(_) = self.state{
                                    if data.action == GridAction::Add {
                                        if data.add_node(pos, data.node_type) {
                                            change_tracker.insert(data.save_stack.last().unwrap().clone());
                                            data.playback_index += 1;
                                        }
                                    } else if data.action == GridAction::Panning {
                                        self.start_pos = *pos;
                                    } else if data.action == GridAction::Remove && option.is_some(){
                                        if data.remove_node(pos) {
                                            change_tracker.insert(data.save_stack.last().unwrap().clone());
                                            data.playback_index += 1;
                                        }
                                    } else if data.action == GridAction::Move && option.is_some() {
                                        self.start_pos = *pos;
                                    }
                                }
                            });
                        info!("Acquire State: {:?}", self.state);
                        info!("Acquire Action: {:?}", data.action);
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
                                    if data.add_node(pos, data.node_type) {
                                        change_tracker.insert(data.save_stack.last().unwrap().clone());
                                        data.playback_index += 1;
                                    }                                    
                                },
                                GridAction::Move => {
                                    if self.start_pos != *pos {
                                        if data.move_node(&self.start_pos, pos) {
                                            change_tracker.insert(data.save_stack.last().unwrap().clone());
                                            data.playback_index += 1;
                                            self.start_pos = *pos;
                                        }
                                    }
                                },
                                GridAction::Remove => {
                                    if option.is_some(){
                                        if data.remove_node(pos) {
                                            change_tracker.insert(data.save_stack.last().unwrap().clone());
                                            data.playback_index += 1;
                                        }
                                    }        
                                },
                                GridAction::Panning => {
                                    // Panning code to be completed
                                },
                                _ => (),
                            }
                        });
                    },
        
                    Event::MouseUp(e) => {
                        if e.button == MouseButton::Right && self.state == GridState::Running(GridAction::Dynamic) && data.action == GridAction::Remove {
                            self.state = GridState::Idle;
                            data.action = GridAction::Dynamic;
                        } else if e.button == MouseButton::Middle && self.state == GridState::Running(GridAction::Dynamic) && data.action == GridAction::Panning {
                            self.state = GridState::Idle;
                            data.action = GridAction::Dynamic;
                        } else if e.button == MouseButton::Left && self.state == GridState::Running(GridAction::Dynamic){
                            self.state = GridState::Idle;
                            data.action = GridAction::Dynamic;
                        } else if e.button == MouseButton::Left {
                            self.state = GridState::Idle;
                        }
                        info!("Release State: {:?}", self.state);
                        info!("Release Action: {:?}", data.action);
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

        if change_tracker.len() != 0 {
            
            // self.playback_index += change_tracker.len();

            info!("Original: Playback index | {:?} vs {:?} | Stack Length", data.playback_index, data.save_stack.len());

            let mut stack_length = data.save_stack.len();
            if data.playback_index != stack_length && stack_length != self.previous_stack_length {
                info!("Previous Stack | {:?} vs {:?} | Current Stack", self.previous_stack_length, stack_length);
                let stack_dif = stack_length - self.previous_stack_length; // Number of elements to stich to the first half of the stack
                let playback_dif = stack_length - data.playback_index + 1; // Number of elements to delete from the middle
                let second_half = data.save_stack.slice(stack_length-stack_dif..);
                data.save_stack.slice(stack_length-playback_dif..);
                data.save_stack.append(second_half);
                stack_length = data.save_stack.len();
                info!("Restich: Playback index | {:?} vs {:?} | Stack Length", data.playback_index, data.save_stack.len());
            }            

            for item in &change_tracker {
                for pos in item.get_positions().iter(){
                    ctx.request_paint_rect(self.invalidation_area(*pos));
                }
            }

            change_tracker.clear();
            self.previous_stack_length = stack_length;
            self.previous_playback_index = data.playback_index;
        }
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &GridWidgetData<T>, _env: &Env) {
        if let LifeCycle::WidgetAdded = event {
            self.previous_stack_length = data.save_stack.len();
            if data.playback_index != 0 {
                ctx.submit_command(UPDATE_GRID_PLAYBACK);
            }
        }
    }

    fn update(
        &mut self,
        ctx: &mut UpdateCtx,
        old_data: &GridWidgetData<T>,
        data: &GridWidgetData<T>,
        _env: &Env,
    ) {       
        if data.show_grid_axis != old_data.show_grid_axis {
            //debug!("Painting the whole window on grid axis change");
            ctx.request_paint();
        }
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
        //Update cell size
        let screen_space: Size = ctx.size();
        //debug!("Screen space: {:?}", ctx.size());

        let width_sized_cell = Size {
            width: screen_space.width / self.max_columns as f64,
            height: screen_space.width / self.max_columns as f64,
        };

        let height_sized_cell = Size {
            width: screen_space.height / self.max_rows as f64,
            height: screen_space.height / self.max_rows as f64,
        };

        self.visible_rows = (screen_space.height / width_sized_cell.height).ceil() as usize;
        self.visible_columns = (screen_space.width / height_sized_cell.width).ceil() as usize;
        self.chosen_cell_size = self.min_cell_size;

        if self.visible_rows > self.max_rows || self.visible_columns > self.max_columns {
            let row_diff = self.visible_rows as i32 - self.max_rows as i32;
            let col_diff = self.visible_columns as i32 - self.max_columns as i32;

            if row_diff > col_diff {
                // Calculate minimum cell size to have all columns
                self.chosen_cell_size = height_sized_cell;
                self.visible_rows = self.max_rows;
                self.visible_columns =
                    (screen_space.width / self.chosen_cell_size.width).ceil() as usize;
            } else {
                // Calculate minimum cell size to have all columns
                self.chosen_cell_size = width_sized_cell;
                self.visible_rows =
                    (screen_space.height / self.chosen_cell_size.height).ceil() as usize;
                self.visible_columns = self.max_columns;
            }
        }

        if self.chosen_cell_size.height < self.min_cell_size.height {
            self.chosen_cell_size = self.min_cell_size;
        }

        //debug!("Visible rows: {:?}", self.visible_rows);
        //debug!("Max rows: {:?}", self.max_rows);
        //debug!("Visible columns: {:?}", self.visible_columns);
        //debug!("Max column:  {:?}", self.max_columns);
        //debug!("Chosen cell size: {:?}", self.chosen_cell_size);
        //debug!("Minimum cell size: {:?}", self.min_cell_size);

        // Draw grid cells

        // Calculate area to render
        let mut paint_rectangles: Vector<Rect> = Vector::new();

        for paint_rect in ctx.region().rects().iter() {
            paint_rectangles.push_front(*paint_rect);
        }

        for paint_rect in paint_rectangles.iter() {
            let from_grid_pos: GridNodePosition = self.grid_pos(paint_rect.origin()).unwrap();
            let from_row = from_grid_pos.row;
            let from_col = from_grid_pos.col;

            let to_grid_pos = self
                .grid_pos(Point::new(paint_rect.max_x(), paint_rect.max_y()))
                .unwrap_or(GridNodePosition {
                    col: self.visible_columns - 1,
                    row: self.visible_rows - 1,
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
                        x: self.chosen_cell_size.width * col as f64,
                        y: self.chosen_cell_size.height * row as f64,
                    };
                    let rect = Rect::from_origin_size(point, self.chosen_cell_size);

                    let grid_pos = GridNodePosition { row, col };

                    if let Some(runner) = data.grid.get(&grid_pos){
                        ctx.fill(rect, runner.get_color());
                    }
                }
            }
        }

        let bounding_box = ctx.region().bounding_box();

        let from_grid_pos: GridNodePosition = self.grid_pos(bounding_box.origin()).unwrap();
        let from_row = from_grid_pos.row;
        let from_col = from_grid_pos.col;

        let to_grid_pos = self
            .grid_pos(Point::new(bounding_box.max_x(), bounding_box.max_y()))
            .unwrap_or(GridNodePosition {
                col: self.visible_columns - 1,
                row: self.visible_rows - 1,
            });
        let to_row = to_grid_pos.row;
        let to_col = to_grid_pos.col;

        // Draw grid axis

        if data.show_grid_axis {
            for row in from_row..=to_row {
                let from_point = Point {
                    x: 0.0,
                    y: self.chosen_cell_size.height * row as f64,
                };

                let size = Size::new(ctx.size().width, self.chosen_cell_size.height * 0.05);
                let rect = Rect::from_origin_size(from_point, size);
                ctx.fill(rect, &gray::GAINSBORO);
            }

            for col in from_col..=to_col {
                let from_point = Point {
                    x: self.chosen_cell_size.width * col as f64,
                    y: 0.0,
                };

                let height = self.visible_rows as f64 * self.chosen_cell_size.height;

                let size = Size::new(self.chosen_cell_size.width * 0.05, height);
                let rect = Rect::from_origin_size(from_point, size);
                ctx.fill(rect, &gray::GAINSBORO);
            }
        }
    }
}
