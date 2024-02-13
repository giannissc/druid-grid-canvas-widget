///////////////////////////////////////////////////////////////////////////////////////////////////
/// 
/// Imports
/// 
///////////////////////////////////////////////////////////////////////////////////////////////////
use std::hash::Hash;
use std::fmt::Debug;
use canvas::Canvas;
use druid::{Data, Color, Size,};
use druid::im::{HashMap, HashSet};
use grid_canvas::{GridCanvas, GridCanvasData, GridChild};

///////////////////////////////////////////////////////////////////////////////////////////////////
/// 
/// Modules
/// 
///////////////////////////////////////////////////////////////////////////////////////////////////
pub mod panning;
pub mod zooming;
pub mod snapping;
pub mod rotation;
pub mod canvas;
pub mod cassette;
pub mod grid_canvas;

///////////////////////////////////////////////////////////////////////////////////////////////////
/// 
/// GridIndex
/// 
///////////////////////////////////////////////////////////////////////////////////////////////////
#[derive(Clone, Data, Copy, PartialEq, Debug, Hash, Eq)]
pub struct GridIndex {
    pub row: isize,
    pub col: isize,
}

impl GridIndex {
    pub fn new(row: isize, col: isize) -> Self {
        Self {
            row,
            col,
        }
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
        [
            above_left,
            above_right,
            below_left,
            below_right,
        ]
    }
}

///////////////////////////////////////////////////////////////////////////////////////////////////
//
// GridItem
//
///////////////////////////////////////////////////////////////////////////////////////////////////
pub trait GridItem: Copy + Clone + Hash + Eq{
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
pub enum GridState{
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
pub enum GridAction{
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
pub enum StackItem<T: GridItem>{
    Add(GridIndex, T, Option<T>),
    Remove(GridIndex, T),
    Move(GridIndex, GridIndex, T),
    BatchAdd(HashMap<GridIndex, (T, Option<T>)>),
    BatchRemove(HashMap<GridIndex, T>),
}

impl<T: GridItem + Debug + PartialEq> StackItem<T> where GridCanvasData<T>: Data{
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

    fn forward_grid(&self, grid: &mut HashMap<GridIndex, T>){
        match self{
            StackItem::Add(grid_index, current_item, _) => {
                grid.insert(*grid_index, *current_item);
            },
            StackItem::Remove(grid_index, _) => {
                grid.remove(grid_index);
            },
            StackItem::Move(from_index, to_index, item) => {
                grid.remove(from_index);
                grid.insert(*to_index, *item);
            },
            StackItem::BatchAdd(items) => {
                for (grid_index, (current_item, _)) in items {
                    grid.insert(*grid_index, *current_item);
                }
            },
            StackItem::BatchRemove(items) => {
                for (grid_index, _) in items {
                    grid.remove(grid_index);
                }
            }
        }
    }

    fn reverse_grid(&self, grid: &mut HashMap<GridIndex, T>){       
        match self{
            StackItem::Add(grid_index, _, previous_item) => {
                grid.remove(grid_index);
                if let Some(previous_item) = previous_item {
                    grid.insert(*grid_index, *previous_item);
                }
            },
            StackItem::Remove(grid_index, previous_item) => {
                grid.insert(*grid_index, *previous_item);
            },
            StackItem::Move(from_index, to_index, item) => {
                grid.remove(to_index);
                grid.insert(*from_index, *item);
            }
            StackItem::BatchAdd(items) => {
                for (grid_index, (_, previous_item)) in items {
                    grid.remove(grid_index);
                    if let Some(previous_item) = previous_item {
                        grid.insert(*grid_index, *previous_item);
                    }
                }
            },
            StackItem::BatchRemove(items) => {
                for (grid_index, previous_item) in items {
                    grid.insert(*grid_index, *previous_item);
                }
            }
        }
    }

    fn forward_canvas(&self, canvas: &mut GridCanvas<T>, data: &GridCanvasData<T>){
        let size = Size::new(data.snap_data.cell_size, data.snap_data.cell_size);
        match self{
            StackItem::Add(grid_index, current_item, _) => {
                let from = data.snap_data.get_grid_position(grid_index.row, grid_index.col);
                let child = GridChild::new(current_item.get_short_text(), current_item.get_color(), size);
                canvas.add_child(child, from.into());
            },
            StackItem::Remove(grid_index, _) => {
                let from = data.snap_data.get_grid_position(grid_index.row, grid_index.col);
                canvas.remove_child(from.into());
            },
            StackItem::Move(from_index, to_index, _) => {
                let from = data.snap_data.get_grid_position(from_index.row, from_index.col);
                let to = data.snap_data.get_grid_position(to_index.row, to_index.col);
                canvas.move_child(from.into(), to.into());
            },
            StackItem::BatchAdd(items) => {
                for (grid_index, (current_item, _)) in items {
                    let child = GridChild::new(current_item.get_short_text(), current_item.get_color(), size);
                    let from = data.snap_data.get_grid_position(grid_index.row, grid_index.col);
                    canvas.add_child(child, from.into());
                }
            },
            StackItem::BatchRemove(items) => {
                for (grid_index, _) in items {
                    let from = data.snap_data.get_grid_position(grid_index.row, grid_index.col);
                    canvas.remove_child(from.into());
                }
            }
        }
    }

    fn reverse_canvas(&self, canvas: &mut GridCanvas<T>, data: &GridCanvasData<T>){   
        let size = Size::new(data.snap_data.cell_size, data.snap_data.cell_size);    
        match self{
            StackItem::Add(grid_index, _, previous_item) => {
                let from = data.snap_data.get_grid_position(grid_index.row, grid_index.col);
                canvas.remove_child(from.into());
                if let Some(previous_item) = previous_item {
                    let child = GridChild::new(previous_item.get_short_text(), previous_item.get_color(), size);
                    canvas.add_child(child, from.into())
                }
            },
            StackItem::Remove(grid_index, previous_item) => {
                let from = data.snap_data.get_grid_position(grid_index.row, grid_index.col);
                let child = GridChild::new(previous_item.get_short_text(), previous_item.get_color(), size);
                canvas.add_child(child, from.into())
            },
            StackItem::Move(from_index, to_index, _) => {
                let from = data.snap_data.get_grid_position(from_index.row, from_index.col);
                let to = data.snap_data.get_grid_position(to_index.row, to_index.col);
                canvas.move_child(from.into(), to.into())
            }
            StackItem::BatchAdd(items) => {
                for (grid_index, (_, previous_item)) in items {
                    let from = data.snap_data.get_grid_position(grid_index.row, grid_index.col);
                    canvas.remove_child(from.into());
                    if let Some(previous_item) = previous_item {
                        let child = GridChild::new(previous_item.get_short_text(), previous_item.get_color(), size);
                        canvas.add_child(child, from.into())
                    }
                }
            },
            StackItem::BatchRemove(items) => {
                for (grid_index, previous_item) in items {
                    let from = data.snap_data.get_grid_position(grid_index.row, grid_index.col);
                    let child = GridChild::new(previous_item.get_short_text(), previous_item.get_color(), size);
                    canvas.add_child(child, from.into());
                }
            }
        }
    }
}