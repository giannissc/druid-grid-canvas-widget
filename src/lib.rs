///////////////////////////////////////////////////////////////////////////////////////////////////
use canvas::Canvas;
use druid::im::{HashMap, HashSet};
use druid::{Color, Data, Size};
use grid_canvas::{GridCanvas, GridCanvasData, GridChild};
use std::fmt::Debug;
///
/// Imports
///
///////////////////////////////////////////////////////////////////////////////////////////////////
use std::hash::Hash;

///////////////////////////////////////////////////////////////////////////////////////////////////

pub mod utils;
pub mod canvas;
pub mod grid_canvas;
///
/// Modules
///
///////////////////////////////////////////////////////////////////////////////////////////////////
pub mod panning;
pub mod rotation;
pub mod snapping;


pub mod zooming;

///////////////////////////////////////////////////////////////////////////////////////////////////
///
/// GridIndex
///
///////////////////////////////////////////////////////////////////////////////////////////////////
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Data,)]
pub struct GridIndex {
    pub row: isize,
    pub col: isize,
}

impl GridIndex {
    pub fn new(row: isize, col: isize) -> Self {
        Self { row, col }
    }
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
        [above_left, above_right, below_left, below_right]
    }
}

///////////////////////////////////////////////////////////////////////////////////////////////////
//
// GridItem
//
///////////////////////////////////////////////////////////////////////////////////////////////////
pub trait GridItem: Copy + Clone + Hash + Eq {
    fn can_add(&self, other: Option<&Self>) -> bool;
    fn can_remove(&self) -> bool;
    fn can_move(&self, other: Option<&Self>) -> bool;
    fn get_color(&self) -> Color;
    fn get_short_text(&self) -> String;
}

///////////////////////////////////////////////////////////////////////////////////////////////////
//
// GridState
//
///////////////////////////////////////////////////////////////////////////////////////////////////
///
#[derive(Clone, Copy, PartialEq, Data, Debug)]
pub enum GridState {
    Idle,
    Running(GridAction),
    Disabled,
}

///////////////////////////////////////////////////////////////////////////////////////////////////
//
// GridAction
//
///////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Clone, Copy, PartialEq, Data, Debug)]
pub enum GridAction {
    Dynamic,
    Add,
    Remove,
    Move,
}

///////////////////////////////////////////////////////////////////////////////////////////////////
//
// StackItem
//
///////////////////////////////////////////////////////////////////////////////////////////////////
#[derive(Clone, PartialEq, Data, Debug, Hash, Eq)]
pub enum StackItem<T: GridItem> {
    Add(GridIndex, T, Option<T>),
    Remove(GridIndex, T),
    Move(GridIndex, GridIndex, T),
    BatchAdd(HashMap<GridIndex, (T, Option<T>)>),
    BatchRemove(HashMap<GridIndex, T>),
}

impl<T: GridItem + Debug + PartialEq> StackItem<T>
where
    GridCanvasData<T>: Data,
{
    fn forward_canvas(&self, canvas: &mut GridCanvas<T>, data: &GridCanvasData<T>) {
        let size = Size::new(data.snap_data.cell_size, data.snap_data.cell_size);
        match self {
            StackItem::Add(grid_index, current_item, _) => {
                let from = data
                    .snap_data
                    .get_grid_position(grid_index.row, grid_index.col);
                let child = GridChild::new(
                    current_item.get_short_text(),
                    current_item.get_color(),
                    size,
                );
                canvas.add_child(child, from.into());
            }
            StackItem::Remove(grid_index, _) => {
                let from = data
                    .snap_data
                    .get_grid_position(grid_index.row, grid_index.col);
                canvas.remove_child(from.into());
            }
            StackItem::Move(from_index, to_index, _) => {
                let from = data
                    .snap_data
                    .get_grid_position(from_index.row, from_index.col);
                let to = data.snap_data.get_grid_position(to_index.row, to_index.col);
                canvas.move_child(from.into(), to.into());
            }
            StackItem::BatchAdd(items) => {
                for (grid_index, (current_item, _)) in items {
                    let child = GridChild::new(
                        current_item.get_short_text(),
                        current_item.get_color(),
                        size,
                    );
                    let from = data
                        .snap_data
                        .get_grid_position(grid_index.row, grid_index.col);
                    canvas.add_child(child, from.into());
                }
            }
            StackItem::BatchRemove(items) => {
                for (grid_index, _) in items {
                    let from = data
                        .snap_data
                        .get_grid_position(grid_index.row, grid_index.col);
                    canvas.remove_child(from.into());
                }
            }
        }
    }

    fn reverse_canvas(&self, canvas: &mut GridCanvas<T>, data: &GridCanvasData<T>) {
        let size = Size::new(data.snap_data.cell_size, data.snap_data.cell_size);
        match self {
            StackItem::Add(grid_index, _, previous_item) => {
                let from = data
                    .snap_data
                    .get_grid_position(grid_index.row, grid_index.col);
                canvas.remove_child(from.into());
                if let Some(previous_item) = previous_item {
                    let child = GridChild::new(
                        previous_item.get_short_text(),
                        previous_item.get_color(),
                        size,
                    );
                    canvas.add_child(child, from.into())
                }
            }
            StackItem::Remove(grid_index, previous_item) => {
                let from = data
                    .snap_data
                    .get_grid_position(grid_index.row, grid_index.col);
                let child = GridChild::new(
                    previous_item.get_short_text(),
                    previous_item.get_color(),
                    size,
                );
                canvas.add_child(child, from.into())
            }
            StackItem::Move(from_index, to_index, _) => {
                let from = data
                    .snap_data
                    .get_grid_position(from_index.row, from_index.col);
                let to = data.snap_data.get_grid_position(to_index.row, to_index.col);
                canvas.move_child(from.into(), to.into())
            }
            StackItem::BatchAdd(items) => {
                for (grid_index, (_, previous_item)) in items {
                    let from = data
                        .snap_data
                        .get_grid_position(grid_index.row, grid_index.col);
                    canvas.remove_child(from.into());
                    if let Some(previous_item) = previous_item {
                        let child = GridChild::new(
                            previous_item.get_short_text(),
                            previous_item.get_color(),
                            size,
                        );
                        canvas.add_child(child, from.into())
                    }
                }
            }
            StackItem::BatchRemove(items) => {
                for (grid_index, previous_item) in items {
                    let from = data
                        .snap_data
                        .get_grid_position(grid_index.row, grid_index.col);
                    let child = GridChild::new(
                        previous_item.get_short_text(),
                        previous_item.get_color(),
                        size,
                    );
                    canvas.add_child(child, from.into());
                }
            }
        }
    }
}
