///////////////////////////////////////////////////////////////////////////////////////////////////
use druid::{
    im::{HashMap, HashSet, Vector},
    widget::{Label, LabelText},
    Affine, BoxConstraints, Color, Data, Env, Event, EventCtx, Insets, LayoutCtx, Lens, LifeCycle,
    LifeCycleCtx, MouseButton, PaintCtx, Point, Rect, RenderContext, Selector, Size, TextAlignment,
    UpdateCtx, Widget, WidgetPod,
};
use druid_color_thesaurus::white;
use log::debug;
///
/// Imports
///
///////////////////////////////////////////////////////////////////////////////////////////////////
use std::{fmt::Debug, time::Instant};

use crate::{
    canvas::{Canvas, Child, PointKey}, snapping::GridSnapData, utils::cassetta::{Cassetta, CassettePlayer, TapeItem}, GridAction, GridIndex, GridItem, GridState,
};

//////////////////////////////////////////////////////////////////////////////////////////////////////
///
/// Command Selectors
///
/////////////////////////////////////////////////////////////////////////////////////////////////////
pub const SET_DISABLED: Selector = Selector::new("disabled-grid-state");
pub const SET_ENABLED: Selector = Selector::new("idle-grid-state");

//////////////////////////////////////////////////////////////////////////////////////
//
// GridWidgetData
//
//////////////////////////////////////////////////////////////////////////////////////
#[derive(Clone, Data, Lens, PartialEq, Debug)]
pub struct GridCanvasData<T: GridItem + PartialEq + Debug> {
    action: GridAction,
    pub grid_item: T,
    pub grid: HashMap<GridIndex, T>,
    // Data Hierarchy
    pub save_data: Cassetta<TapeItem<GridIndex, T>>,
    pub snap_data: GridSnapData,
}

impl<T: GridItem + PartialEq + Debug> GridCanvasData<T>
where
    GridCanvasData<T>: Data,
{
    pub fn new(item_type: T) -> Self {
        Self {
            action: GridAction::Dynamic,
            grid_item: item_type,
            grid: HashMap::new(),
            save_data: Cassetta::new(),
            snap_data: GridSnapData::new(15.0),
        }
    }

    pub fn set_cell_size(&mut self, cell_size: f64) {
        self.snap_data.cell_size = cell_size;
    }

    // Basic Grid methods
    fn add_node(&mut self, pos: &GridIndex, item: T) -> bool {
        self.save_data.clear_delta();
        let option = self.grid.get(pos);

        let command_item;
        if option.is_none() {
            command_item = TapeItem::Add(*pos, item, None);
        } else {
            command_item = TapeItem::Add(*pos, item, Some(*option.unwrap()));
        }

        if item.can_add(option) {
            self.grid.insert(*pos, item);
            self.save_data.insert_and_play(command_item);
            return true;
        }
        false
    }

    fn remove_node(&mut self, pos: &GridIndex) -> bool {
        self.save_data.clear_delta();
        if let Some(item) = self.grid.remove(pos) {
            if item.can_remove() {
                let command_item = TapeItem::Remove(*pos, item);
                self.save_data.insert_and_play(command_item);
                return true;
            } else {
                self.grid.insert(*pos, item);
            }
        }
        false
    }
    fn move_node(&mut self, from: &GridIndex, to: &GridIndex) -> bool {
        self.save_data.clear_delta();
        let item = self.grid.get(from).unwrap();
        let other = self.grid.get(to);
        if item.can_move(other) {
            let item = self.grid.remove(from).unwrap();
            self.grid.insert(*to, item);
            let command_item = TapeItem::Move(*from, *to, item);
            self.save_data.insert_and_play(command_item);
            return true;
        }
        false
    }

    // Auxiliary Grid Methods
    pub fn add_node_perimeter(&mut self, pos: GridIndex, row_n: isize, column_n: isize, tool: T) {
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

        for (pos, (current_item, _)) in &map {
            self.grid.insert(*pos, *current_item);
        }
        self.save_data.insert_and_play(TapeItem::BatchAdd(map));
        // ctx.submit_command(Command::new(TRIGGER_CHANGE, (), Target::Widget(id)));
    }

    // Clear Grid methods
    pub fn clear_all(&mut self) {
        self.save_data
            .insert_and_play(TapeItem::BatchRemove(self.grid.clone()));
        self.grid.clear();
        // ctx.submit_command(Command::new(TRIGGER_CHANGE, (), Target::Widget(id)));
    }
    pub fn clear_except(&mut self, set: HashSet<T>) {
        let mut map: HashMap<GridIndex, T> = HashMap::new();
        for item_type in set {
            self.grid.retain(|pos, item| {
                if *item == item_type {
                    true
                } else {
                    map.insert(*pos, *item);
                    false
                }
            })
        }
        self.save_data.insert_and_play(TapeItem::BatchRemove(map));
    }
    pub fn clear_only(&mut self, set: HashSet<T>) {
        let mut map: HashMap<GridIndex, T> = HashMap::new();
        for item_type in set {
            self.grid.retain(|pos, item| {
                if *item == item_type {
                    map.insert(*pos, *item);
                    false
                } else {
                    true
                }
            })
        }
        self.save_data.insert_and_play(TapeItem::BatchRemove(map));
    }

    // Save stack methods
    fn validate_stack_list(
        &mut self,
        list: Vector<TapeItem<GridIndex, T>>,
    ) -> (HashMap<GridIndex, T>, Vector<TapeItem<GridIndex, T>>) {
        let mut stack_list = Vector::new();
        let mut pos_map = HashMap::new();

        for stack_item in list {
            match stack_item {
                TapeItem::Add(pos, current_item, _) => {
                    let option = self.grid.get(&pos);
                    if current_item.can_add(option) {
                        stack_list.push_back(stack_item);
                        pos_map.insert(pos, current_item);
                    }
                }
                TapeItem::BatchAdd(mut map) => {
                    map.retain(|pos, (current_item, _)| {
                        let option = self.grid.get(pos);
                        if current_item.can_add(option) {
                            pos_map.insert(*pos, *current_item);
                        }
                        current_item.can_add(option)
                    });

                    if !map.is_empty() {
                        stack_list.push_back(TapeItem::BatchAdd(map));
                    }
                }
                _ => (),
            }
        }
        (pos_map, stack_list)
    }

    pub fn submit_to_stack(&mut self, list: Vector<TapeItem<GridIndex, T>>) {
        let (_, save_list) = self.validate_stack_list(list);
        self.save_data.append(save_list);
    }

    pub fn submit_to_stack_and_process(&mut self, list: Vector<TapeItem<GridIndex, T>>) {
        let (pos_map, save_list) = self.validate_stack_list(list);
        for (pos, item) in pos_map.iter() {
            self.grid.insert(*pos, *item);
        }
        self.save_data.append_and_play(save_list);
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
pub struct GridCanvas<T: GridItem + PartialEq + Debug>
where
    GridCanvasData<T>: Data,
{
    start_pos: GridIndex,
    state: GridState,
    // canvas: WidgetPod<GridCanvasData<T>, Canvas<GridCanvasData<T>>>,
    canvas: Canvas<GridCanvasData<T>>,
}

impl<T: Clone + GridItem + Debug> GridCanvas<T>
where
    GridCanvasData<T>: Data,
{
    pub fn new() -> Self {
        let canvas = Canvas::new();
        GridCanvas {
            start_pos: GridIndex { row: 0, col: 0 },
            state: GridState::Idle,
            // canvas: WidgetPod::new(canvas),
            canvas,
        }
    }

    pub fn invalidation_area(&self, pos: GridIndex, cell_size: f64) -> Rect {
        let point = Point {
            x: cell_size * pos.col as f64,
            y: cell_size * pos.row as f64,
        };
        Rect::from_origin_size(
            point,
            Size {
                width: cell_size,
                height: cell_size,
            },
        )
    }

    // For index based layout containers the position will be replaced by an index
    // Might need two variants for this: add and add_relocate in case you don't want
    // to remove the the exist at the to position. Useful for drag and drop between
    // different containers
    // A third method
    pub fn add_child(&mut self, child: impl Widget<GridCanvasData<T>> + 'static, from: PointKey) {
        let canvas =  &mut self.canvas;
        let delete_index = canvas.position_map.remove(&from);

        if let Some(delete_index) = delete_index {
            let last_index = canvas.children.len() - 1;
            let child = canvas.children.remove(last_index);
            if last_index != delete_index {
                // Update position map
                if let Child::Explicit { position, .. } = &child {
                    let key: PointKey = <Point as Into<PointKey>>::into(*position);
                    canvas.position_map.remove(&key);
                    canvas.position_map.insert(key, delete_index);
                }
                canvas.children.remove(delete_index);
                canvas.children.insert(delete_index, child);
            }
        }

        let inner: WidgetPod<GridCanvasData<T>, Box<dyn Widget<GridCanvasData<T>>>> =
            WidgetPod::new(Box::new(child));
        let index = canvas.children.len();
        canvas.children.insert(
            index,
            Child::Explicit {
                inner,
                position: from.clone().into(),
            },
        );
        canvas.position_map.insert(from, index);
    }

    // For index based layout containers the position will be replaced by an index
    pub fn remove_child(&mut self, from: PointKey) {
        // Swap item at index with last item and then delete
        let canvas = &mut self.canvas;
        let delete_index = canvas.position_map.remove(&from);
        let last_index = canvas.children.len() - 1;
        if let Some(delete_index) = delete_index {
            let child = canvas.children.remove(last_index);
            if last_index != delete_index {
                // Update position map
                if let Child::Explicit { position, .. } = &child {
                    let key: PointKey = <Point as Into<PointKey>>::into(*position);
                    canvas.position_map.remove(&key);
                    canvas.position_map.insert(key, delete_index);
                }
                canvas.children.remove(delete_index);
                canvas.children.insert(delete_index, child);
            }
        }
    }

    // For index based layout containers the position will be replaced by an index
    // Might need two variants for this: move and move_relocate in case you don't want
    // to remove the the exist at the to position. Useful for drag and drop within the
    // same container
    pub fn move_child(&mut self, from: PointKey, to: PointKey) {
        let canvas = &mut self.canvas;
        let index_from = canvas.position_map.remove(&from);

        if let Some(old_index) = index_from {
            let inner = canvas.children.remove(old_index);
            match inner {
                Child::Explicit { inner, .. } => {
                    let index = canvas.children.len();
                    canvas.children.insert(
                        index,
                        Child::Explicit {
                            inner,
                            position: to.clone().into(),
                        },
                    );
                    canvas.position_map.insert(to, index);
                }
                _ => panic!(),
            }
        }
    }

    fn advance(&mut self, item: TapeItem<GridIndex, T>, data: &GridCanvasData<T>) {
        let size = Size::new(data.snap_data.cell_size, data.snap_data.cell_size);
        match item {
            TapeItem::Add(grid_index, item, _) => {
                let from: PointKey = data.snap_data.get_grid_position(grid_index.row, grid_index.col).into();
                let child = GridChild::new(
                    item.get_short_text(),
                    item.get_color(),
                    size,
                );
                self.add_child(child, from);
            },
            TapeItem::Remove(grid_index, _) => {
                let from: PointKey = data.snap_data.get_grid_position(grid_index.row, grid_index.col).into();
                self.remove_child(from);
            },
            TapeItem::Move(from_grid_index, to_grid_index, _) => {
                let from: PointKey = data.snap_data.get_grid_position(from_grid_index.row, from_grid_index.col).into();
                let to: PointKey = data.snap_data.get_grid_position(to_grid_index.row, to_grid_index.col).into();
                self.move_child(from, to);
            },
            TapeItem::BatchAdd(items) => {
                for (grid_index, (item, _)) in items.into_iter(){
                    let from: PointKey = data.snap_data.get_grid_position(grid_index.row, grid_index.col).into();
                    let child = GridChild::new(
                        item.get_short_text(),
                        item.get_color(),
                        size,
                    );
                    self.add_child(child, from);
                }
            },
            TapeItem::BatchRemove(items) => {
                for (grid_index, _) in items{
                    let from: PointKey = data.snap_data.get_grid_position(grid_index.row, grid_index.col).into();
                    self.remove_child(from);
                }
            },
            
        }
    }

    fn rewind(&mut self, item: TapeItem<GridIndex, T>, data: &GridCanvasData<T>) {
        let size = Size::new(data.snap_data.cell_size, data.snap_data.cell_size);
        match item {
            TapeItem::Add(grid_index, _, previous_item) => {
                let from: PointKey = data.snap_data.get_grid_position(grid_index.row, grid_index.col).into();
                self.remove_child(from.clone());
                if let Some(item) = previous_item {
                    let child = GridChild::new(
                        item.get_short_text(),
                        item.get_color(),
                        size,
                    );
                    self.add_child(child, from);
                }
            },
            TapeItem::Remove(grid_index, previous_item) => {
                let from: PointKey = data.snap_data.get_grid_position(grid_index.row, grid_index.col).into();
                let child = GridChild::new(
                    previous_item.get_short_text(),
                    previous_item.get_color(),
                    size,
                );
                self.add_child(child, from);
            },
            TapeItem::Move(from_grid_index, to_grid_index, _) => {
                let from: PointKey = data.snap_data.get_grid_position(from_grid_index.row, from_grid_index.col).into();
                let to: PointKey = data.snap_data.get_grid_position(to_grid_index.row, to_grid_index.col).into();
                self.move_child(to, from);
            },
            TapeItem::BatchAdd(items) => {
                for (grid_index, (_, previous_item)) in items.into_iter(){
                    let from: PointKey = data.snap_data.get_grid_position(grid_index.row, grid_index.col).into();
                    self.remove_child(from.clone());
                    if let Some(item) = previous_item{
                        let child = GridChild::new(
                            item.get_short_text(),
                            item.get_color(),
                            size,
                        );
                        self.add_child(child, from);
                    }
                }
            },
            TapeItem::BatchRemove(items) => {
                for (grid_index, item) in items{
                    let from: PointKey = data.snap_data.get_grid_position(grid_index.row, grid_index.col).into();
                    let child = GridChild::new(
                        item.get_short_text(),
                        item.get_color(),
                        size,
                    );
                    self.add_child(child, from);
                }
            },
            
        }
    }
}

impl<T: GridItem + PartialEq + Debug> Widget<GridCanvasData<T>> for GridCanvas<T>
where
    GridCanvasData<T>: Data,
{
    fn event(
        &mut self,
        ctx: &mut EventCtx,
        event: &Event,
        data: &mut GridCanvasData<T>,
        env: &Env,
    ) {
        // println!("Canvas Wrapper Event");
        match &self.state {
            GridState::Idle => {
                // info!("Idle State");
                match event {
                    Event::Command(cmd) => {
                        if cmd.is(SET_DISABLED) {
                            self.state = GridState::Disabled;
                        }
                    }
                    Event::MouseDown(e) => {
                        let (row, col) = data.snap_data.get_grid_index(e.pos);
                        let grid_index = GridIndex::new(row, col);
                        let option = data.grid.get(&grid_index);

                        if self.state == GridState::Idle {
                            if e.button == MouseButton::Left {
                                // info!("Left Click");
                                // info!("Start State: {:?}", self.state);
                                // info!("Start Action: {:?}", data.action);
                                match data.action {
                                    GridAction::Dynamic => {
                                        self.state = GridState::Running(GridAction::Dynamic);
                                        match option {
                                            None => {
                                                data.action = GridAction::Add;
                                            }
                                            Some(item) => {
                                                if *item == data.grid_item {
                                                    data.action = GridAction::Move
                                                } else {
                                                    data.action = GridAction::Add
                                                }
                                            }
                                        }
                                    }
                                    GridAction::Move => {
                                        if option.is_some() {
                                            self.state = GridState::Running(GridAction::Move);
                                        }
                                    }
                                    _ => {
                                        self.state = GridState::Running(data.action);
                                    }
                                }
                            } else if e.button == MouseButton::Right {
                                // info!("Right Click");
                                if let GridAction::Dynamic = data.action {
                                    self.state = GridState::Running(data.action);
                                    data.action = GridAction::Remove;
                                }
                            }
                        }

                        if let GridState::Running(_) = self.state {
                            if data.action == GridAction::Add {
                                data.add_node(&grid_index, data.grid_item);
                            } else if data.action == GridAction::Remove && option.is_some() {
                                data.remove_node(&grid_index);
                            } else if data.action == GridAction::Move && option.is_some() {
                                self.start_pos = grid_index;
                            }
                        }
                        // info!("Acquire State: {:?}", self.state);
                        // info!("Acquire Action: {:?}", data.action);
                    }

                    _ => {}
                }
            }
            GridState::Running(_) => {
                // info!("Running State");
                match event {
                    Event::MouseMove(e) => {
                        let (row, col) = data.snap_data.get_grid_index(e.pos);
                        let grid_index = GridIndex::new(row, col);
                        let option = data.grid.get(&grid_index);

                        match data.action {
                            GridAction::Add => {
                                data.add_node(&grid_index, data.grid_item);
                            }
                            GridAction::Move => {
                                if self.start_pos != grid_index {
                                    if data.move_node(&self.start_pos, &grid_index) {
                                        self.start_pos = grid_index;
                                    }
                                }
                            }
                            GridAction::Remove => {
                                if option.is_some() {
                                    data.remove_node(&grid_index);
                                }
                            }
                            _ => (),
                        }
                    }

                    Event::MouseUp(e) => {
                        if e.button == MouseButton::Right
                            && self.state == GridState::Running(GridAction::Dynamic)
                            && data.action == GridAction::Remove
                        {
                            self.state = GridState::Idle;
                            data.action = GridAction::Dynamic;
                        } else if e.button == MouseButton::Left
                            && self.state == GridState::Running(GridAction::Dynamic)
                        {
                            self.state = GridState::Idle;
                            data.action = GridAction::Dynamic;
                        } else if e.button == MouseButton::Left {
                            self.state = GridState::Idle;
                        }
                        // info!("Release State: {:?}", self.state);
                        // info!("Release Action: {:?}", data.action);
                    }
                    _ => {}
                }
            }
            GridState::Disabled => {
                if let Event::Command(cmd) = event {
                    if cmd.is(SET_ENABLED) {
                        self.state = GridState::Idle;
                    }
                }
            }
        }
        self.canvas.event(ctx, event, data, env);
    }

    fn lifecycle(
        &mut self,
        ctx: &mut LifeCycleCtx,
        event: &LifeCycle,
        data: &GridCanvasData<T>,
        env: &Env,
    ) {
        // println!("Canvas Wrapper ({:?}) Lifecycle: {:?}", ctx.widget_id(), event);
        // TODO: Handle ViewContext Changed
        if let LifeCycle::WidgetAdded = event {
            for (grid_index, item) in data.grid.iter() {
                let from = data.snap_data.get_grid_position(grid_index.row, grid_index.col);
                let size = Size::new(data.snap_data.cell_size, data.snap_data.cell_size);
                let child = GridChild::new(
                    item.get_short_text(),
                    item.get_color(),
                    size,
                );
                self.add_child(child, from.into())
            }
            ctx.children_changed();
        }

        self.canvas.lifecycle(ctx, event, data, env);
    }

    fn update(
        &mut self,
        ctx: &mut UpdateCtx,
        old_data: &GridCanvasData<T>,
        data: &GridCanvasData<T>,
        env: &Env,
    ) {
        self.canvas.update(ctx, old_data, data, env);
        // self.canvas.update(ctx, data, env);
        debug!("\n{:?}",Instant::now());
        debug!("add item: {:?}", data.save_data.add_delta);
        for item in data.save_data.add_delta.iter(){
            self.advance(item.clone(), data);
            ctx.children_changed();
            ctx.request_paint();
            
        }

        debug!("delete item: {:?}", data.save_data.remove_delta);
        for item in data.save_data.remove_delta.iter(){
            self.rewind(item.clone(), data);
            ctx.children_changed();
            ctx.request_paint();
        }


        if old_data.snap_data.pan_data.offset != data.snap_data.pan_data.offset
            || old_data.snap_data.zoom_data.zoom_scale != data.snap_data.zoom_data.zoom_scale
        {
            ctx.request_layout()
        }
    }

    fn layout(
        &mut self,
        ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        data: &GridCanvasData<T>,
        env: &Env,
    ) -> Size {
        // let origin = Point::new(0., 0.);
        //debug!("Box constraints width: {:?}", bc.max().width);
        //debug!("Box constraints height: {:?}", bc.max().height);
        self.canvas.offset = data.snap_data.pan_data.offset;
        self.canvas.scale = data.snap_data.zoom_data.zoom_scale;
        self.canvas.layout(ctx, bc, data, env);

        // self.canvas.set_origin(ctx, data.snap_data.pan_data.absolute_offset);

        bc.max()
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &GridCanvasData<T>, env: &Env) {
        //debug!("Running paint method");
        // Draw grid cells

        // let damage_region = ctx.region().clone();
        // Calculate area to render
        // let paint_rectangles = damage_region.rects();

        ctx.with_save(|ctx| {
            let scale = Affine::scale(data.snap_data.zoom_data.zoom_scale);

            // ctx.transform(translate);
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

impl<T: Data> GridChild<T> {
    pub fn new(text: impl Into<LabelText<T>>, color: Color, size: Size) -> Self {
        // let foo = Label::new(tooltip_text).tooltip();
        let mut label_text = Label::new(text);
        label_text.set_line_break_mode(druid::widget::LineBreaking::WordWrap);
        label_text.set_text_color(white::ALABASTER);
        label_text.set_text_size(size.width / 3.3);
        label_text.set_text_alignment(TextAlignment::Center);

        GridChild {
            label_text,
            label_size: Size::ZERO,
            color,
            size,
        }
    }
}

impl<T: Data> Widget<T> for GridChild<T> {
    fn event(&mut self, _ctx: &mut EventCtx, _event: &Event, _data: &mut T, _env: &Env) {
        // Add tooltip logic on hover
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &T, env: &Env) {
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
        ctx.request_paint();
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
