use std::marker::PhantomData;

use druid::{BoxConstraints, Data, Env, Event, EventCtx, LayoutCtx, Lens, LifeCycle,
    LifeCycleCtx, PaintCtx, RenderContext, UpdateCtx, Widget, Selector, Point, Rect, Size, Color, MouseButton};

use druid::im::{HashMap, Vector};

use druid_color_thesaurus::*;

//////////////////////////////////////////////////////////////////////////////////////////////////////
/// 
/// Command Selectors
/// 
/////////////////////////////////////////////////////////////////////////////////////////////////////

pub const SET_DISABLED: Selector = Selector::new("disabled-grid-state");
pub const SET_ENABLED: Selector = Selector::new("idle-grid-state");
pub const RESET: Selector = Selector::new("RESET");
pub const SUBMIT_COMMAND_QUEUE: Selector = Selector::new("CLEAR");


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
    pub fn neighbors_octilinear(self) -> [GridNodePosition; 8] {
        let above = self.above();
        let below = self.below();
        let left = self.left();
        let right = self.right();
        let above_left = above.left();
        let above_right = above.right();
        let below_left = below.left();
        let below_right = below.right();
        [
            above,
            below,
            left,
            right,
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
pub trait GridRunner: Clone{
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
#[derive(Clone, PartialEq, Data, Debug)]
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

#[derive(Clone, PartialEq, Data, Debug)]
pub enum GridAction{
    Dynamic,
    Add,
    Remove,
    Move,
    Panning,
}

//////////////////////////////////////////////////////////////////////////////////////
//
// CommandItem
//
//////////////////////////////////////////////////////////////////////////////////////
#[derive(Clone, PartialEq, Data, Debug)]
pub enum CommandItem<T>{
    Add(GridNodePosition, T),
    Remove(GridNodePosition, T),
    Move(GridNodePosition, GridNodePosition, T),
}

//////////////////////////////////////////////////////////////////////////////////////
//
// GridWidgetData
//
//////////////////////////////////////////////////////////////////////////////////////
#[derive(Clone, PartialEq, Data, Lens)]
pub struct GridWidgetData<T:GridRunner + PartialEq>{
    grid: HashMap<GridNodePosition, T>,
    command_queue: Vector<CommandItem<T>>,
    pub show_grid_axis: bool,
    pub action: GridAction,
    pub node_type: T,
}

impl<T:GridRunner + PartialEq> GridWidgetData<T>{
    pub fn new(initial_node: T) -> Self {
        GridWidgetData {
            grid: HashMap::new(),
            command_queue: Vector::new(),
            show_grid_axis: true,
            action: GridAction::Dynamic,
            node_type: initial_node,
        }
    }

    fn add_node(&mut self, pos: &GridNodePosition, item: T){
        let option = self.grid.get(pos);
        if item.can_add(option){
            self.grid.insert(*pos, item.clone());
            let command_item = CommandItem::Add(*pos, item.clone());
            self.command_queue.push_back(command_item);
        } 
    }

    fn remove_node(&mut self, pos: &GridNodePosition) -> Option<T>{
        let option = self.grid.remove(pos);
        match option{
            None => (),
            Some(item) => {
                if item.can_remove(){
                    let command_item = CommandItem::Remove(*pos, item.clone());
                    self.command_queue.push_back(command_item);
                    return Some(item);
                } else {
                    self.grid.insert(*pos, item);
                }
            }
        }
        return None;
    }

    fn move_node(&mut self, from: &GridNodePosition, to:&GridNodePosition) -> bool {
        let item = self.grid.get(from).unwrap();
        let other = self.grid.get(to);
        if item.can_move(other) {
            let item = self.grid.remove(from).unwrap();
            self.grid.insert(*to, item.clone());
            let command_item = CommandItem::Move(*from, *to, item);
            self.command_queue.push_back(command_item);
            return true;
        }
        return false;
    }

    fn submit_command_queue(&mut self) {
        self.command_queue.clear();
    }

    fn clear_all(&mut self) {}
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
    current_pos: GridNodePosition,
    state: GridState,
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
            current_pos: GridNodePosition { row: 0, col: 0 },
            state: GridState::Idle,
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
    fn event(&mut self, _ctx: &mut EventCtx, event: &Event, data: &mut GridWidgetData<T>, _env: &Env) {
        match &self.state{
            GridState::Idle => {
                //println!("Idle State");
                match event {
                    Event::Command(cmd) => {
                        if cmd.is(SET_DISABLED) {
                            self.state = GridState::Disabled;
                        } else if cmd.is(RESET) {
                            data.clear_all();
                        } else if cmd.is(SUBMIT_COMMAND_QUEUE) {
                            data.submit_command_queue();
                        }
                    },
        
                    Event::MouseDown(e) => {
                        let grid_pos_opt = self.grid_pos(e.pos);
                            grid_pos_opt.iter().for_each(|pos| {
                                let option = data.grid.get(pos);
                                if self.state == GridState::Idle{
                                    if e.button == MouseButton::Left{
                                        println!("Left Click");
                                        println!("Start State: {:?}", self.state);
                                        println!("Start Action: {:?}", data.action);
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
                                                if option != Option::None {
                                                    self.state = GridState::Running(GridAction::Move);
                                                }
                                            },
                                            _ => {
                                                self.state = GridState::Running(data.action.clone());
                                            },                                        
                                        }

                                    } else if e.button == MouseButton::Right{
                                        println!("Right Click");
                                        match data.action{
                                            GridAction::Dynamic => {
                                                self.state = GridState::Running(data.action.clone());
                                                data.action = GridAction::Remove;
        
                                            },
                                            _ => (),
                                        }

                                    } else if e.button == MouseButton::Middle{
                                        println!("Middle Click");
                                        match data.action{
                                            GridAction::Dynamic => {
                                                self.state = GridState::Running(data.action.clone());
                                                data.action = GridAction::Panning;
                                            },
                                            _ => (),
                                        }

                                    }
                                }

                                match self.state{
                                    GridState::Running(_) => {
                                        if data.action == GridAction::Add {
                                            data.add_node(pos, data.node_type.clone());
                                        } else if data.action == GridAction::Panning {
                                            self.current_pos = *pos;
                                        } else if data.action == GridAction::Remove && option != Option::None{
                                            data.remove_node(pos);
                                        } else if data.action == GridAction::Move && option != Option::None {
                                            self.current_pos = *pos;
                                        }
                                    },
                                    _ => (),
                                }
                            });
                        println!("Acquire State: {:?}", self.state);
                        println!("Acquire Action: {:?}", data.action);
                    },

                    _ => {},
                }
            },
            GridState::Running(_) => {
                //println!("Running State");
                match event {            
                    Event::MouseMove(e) => {
                        let grid_pos_opt = self.grid_pos(e.pos);
                        grid_pos_opt.iter().for_each(|pos| {
                            let option = data.grid.get(pos);
                            match data.action{
                                GridAction::Add => {
                                    data.add_node(pos, data.node_type.clone())
                                },
                                GridAction::Move => {
                                    if self.current_pos != *pos {
                                        if data.move_node(&self.current_pos, pos) {self.current_pos = *pos;}
                                    }
                                },
                                GridAction::Remove => {
                                    if option != Option::None{
                                        data.remove_node(pos);
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
                        println!("Release State: {:?}", self.state);
                        println!("Release Action: {:?}", data.action);
                    },
                    _ => {},
                }
            },
            GridState::Disabled => {
                match event {      
                    Event::Command(cmd) => {
                        if cmd.is(SET_ENABLED) {
                            self.state = GridState::Idle;
                        }
                    },      
                    _ => {},
                }
            },
        }

        
    }

    fn lifecycle(&mut self, _ctx: &mut LifeCycleCtx, _event: &LifeCycle, _data: &GridWidgetData<T>, _env: &Env) {
    }

    fn update(
        &mut self,
        ctx: &mut UpdateCtx,
        old_data: &GridWidgetData<T>,
        data: &GridWidgetData<T>,
        _env: &Env,
    ) {
        //debug!("Running grid widget update method");
        //debug!("Difference: {:?}", data.grid.get_storage().difference(old_data.grid.get_storage()));

        if data.show_grid_axis != old_data.show_grid_axis {
            //debug!("Painting the whole window on grid axis change");
            ctx.request_paint();
        } else {
            for cell in data.command_queue.iter() {
                match cell{
                    CommandItem::Add(pos, node) => ctx.request_paint_rect(self.invalidation_area(*pos)),
                    CommandItem::Remove(pos, node) => ctx.request_paint_rect(self.invalidation_area(*pos)),
                    CommandItem::Move(from, to, node) => {
                        ctx.request_paint_rect(self.invalidation_area(*from));
                        ctx.request_paint_rect(self.invalidation_area(*to));
                    }
                }
            }
            ctx.submit_command(SUBMIT_COMMAND_QUEUE);
        }
    }

    fn layout(
        &mut self,
        _layout_ctx: &mut LayoutCtx,
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

                    match data.grid.get(&grid_pos){
                        None => (),
                        Some(runner) => ctx.fill(rect, runner.get_color())
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
