///////////////////////////////////////////////////////////////////////////////////////////////////
/// 
/// Imports
/// 
///////////////////////////////////////////////////////////////////////////////////////////////////
use std::hash::Hash;
use druid::{Data, Color,};
use druid::im::{HashMap, HashSet};

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
pub mod save_system;
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