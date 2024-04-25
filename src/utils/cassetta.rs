///////////////////////////////////////////////////////////////////////////////////////////////////
///
/// Imports
///
///////////////////////////////////////////////////////////////////////////////////////////////////
use druid::{
    im::{HashMap, Vector},
    Data, Lens,
};
use std::{fmt::Debug, hash::Hash};

///////////////////////////////////////////////////////////////////////////////////////////////////
///
/// SaveSystemData
///
///////////////////////////////////////////////////////////////////////////////////////////////////
#[derive(Clone, Data, Lens, PartialEq, Debug)]
pub struct Cassetta<T: Clone + Debug> {
    pub undo_tape: Vector<T>,
    pub redo_tape: Vector<T>,
}

impl<T: Clone + Debug> Cassetta<T> {
    pub fn new() -> Self {
        Self {
            undo_tape: Vector::new(),
            redo_tape: Vector::new(),
        }
    }

    pub fn insert(&mut self, item: T) {
        self.redo_tape.clear();
        self.redo_tape.push_back(item);
    }

    pub fn insert_and_play(&mut self, item: T) {
        self.undo_tape.push_back(item);
        self.redo_tape.clear();
    }

    pub fn append(&mut self, other: Vector<T>) {
        self.redo_tape.clear();
        self.redo_tape.append(other);
    }

    pub fn append_and_play(&mut self, other: Vector<T>) {
        self.undo_tape.append(other);
        self.redo_tape.clear();
    }

    pub fn undo(&mut self) -> Option<T> {
        let item = self.undo_tape.pop_back();
        if let Some(item) = item.clone() {
            self.redo_tape.push_back(item);
        }
        item
    }

    pub fn redo(&mut self) -> Option<T> {
        let item = self.redo_tape.pop_back();
        if let Some(item) = item.clone() {
            self.undo_tape.push_back(item);
        }
        item
    }
}

#[derive(Clone, Debug, PartialEq, Data)]
pub enum TapeItem<K, V> where K: Clone + Debug + Hash + Eq{
    Add(K, V, Option<V>),
    Remove(K, V),
    Move(K, K, V),
    BatchAdd(HashMap<K, (V, Option<V>)>),
    BatchRemove(HashMap<K, V>),
    // BatchMove(HashMap<K, (K, V)>)
}

pub trait CassettePlayer<K, V> where K: Clone + Debug + Hash + Eq{
    fn advance(&mut self, item: TapeItem<K, V>);
    fn rewind(&mut self, item: TapeItem<K, V>);
}

impl<K: Eq + Clone + Hash + Debug, V: Clone> CassettePlayer<K, V> for HashMap<K, V> {
    fn advance(&mut self, item: TapeItem<K, V>) {
        match item {
            TapeItem::Add(key, current_item, _) => {
                self.insert(key, current_item);
            }
            TapeItem::Remove(grid_index, _) => {
                self.remove(&grid_index);
            }
            TapeItem::Move(from_index, to_index, item) => {
                self.remove(&from_index);
                self.insert(to_index, item);
            }
            TapeItem::BatchAdd(items) => {
                for (grid_index, (current_item, _)) in items {
                    self.insert(grid_index, current_item);
                }
            }
            TapeItem::BatchRemove(items) => {
                for (grid_index, _) in items {
                    self.remove(&grid_index);
                }
            }
        }
    }

    fn rewind(&mut self, item: TapeItem<K, V>) {
        match item {
            TapeItem::Add(grid_index, _, previous_item) => {
                self.remove(&grid_index);
                if let Some(previous_item) = previous_item {
                    self.insert(grid_index, previous_item);
                }
            }
            TapeItem::Remove(grid_index, previous_item) => {
                self.insert(grid_index, previous_item);
            }
            TapeItem::Move(from_index, to_index, item) => {
                self.remove(&to_index);
                self.insert(from_index, item);
            }
            TapeItem::BatchAdd(items) => {
                for (grid_index, (_, previous_item)) in items {
                    self.remove(&grid_index);
                    if let Some(previous_item) = previous_item {
                        self.insert(grid_index, previous_item);
                    }
                }
            }
            TapeItem::BatchRemove(items) => {
                for (grid_index, previous_item) in items {
                    self.insert(grid_index, previous_item);
                }
            }
        }
    }
}

impl<V: Clone> CassettePlayer<usize, V> for Vector<V> {
    fn advance(&mut self, item: TapeItem<usize, V>) {
        match item {
            TapeItem::Add(key, current_item, _) => {
                self.insert(key, current_item);
            }
            TapeItem::Remove(grid_index, _) => {
                self.remove(grid_index);
            }
            TapeItem::Move(from_index, to_index, item) => {
                self.remove(from_index);
                self.insert(to_index, item);
            }
            TapeItem::BatchAdd(items) => {
                for (grid_index, (current_item, _)) in items {
                    self.insert(grid_index, current_item);
                }
            }
            TapeItem::BatchRemove(items) => {
                for (grid_index, _) in items {
                    self.remove(grid_index);
                }
            }
        }
    }

    fn rewind(&mut self, item: TapeItem<usize, V>) {
        match item {
            TapeItem::Add(grid_index, _, previous_item) => {
                self.remove(grid_index);
                if let Some(previous_item) = previous_item {
                    self.insert(grid_index, previous_item);
                }
            }
            TapeItem::Remove(grid_index, previous_item) => {
                self.insert(grid_index, previous_item);
            }
            TapeItem::Move(from_index, to_index, item) => {
                self.remove(to_index);
                self.insert(from_index, item);
            }
            TapeItem::BatchAdd(items) => {
                for (grid_index, (_, previous_item)) in items {
                    self.remove(grid_index);
                    if let Some(previous_item) = previous_item {
                        self.insert(grid_index, previous_item);
                    }
                }
            }
            TapeItem::BatchRemove(items) => {
                for (grid_index, previous_item) in items {
                    self.insert(grid_index, previous_item);
                }
            }
        }
    }
}
