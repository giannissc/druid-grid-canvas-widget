///////////////////////////////////////////////////////////////////////////////////////////////////
/// 
/// Imports
/// 
///////////////////////////////////////////////////////////////////////////////////////////////////
use druid::{im::Vector, Data, Lens};
use std::fmt::Debug;

///////////////////////////////////////////////////////////////////////////////////////////////////
/// 
/// SaveSystemData
/// 
///////////////////////////////////////////////////////////////////////////////////////////////////
#[derive(Clone, Data, Lens, PartialEq, Debug)]
pub struct Cassette<T: Clone + Debug> {
    pub tape: Vector<T>,
    pub playback_index: usize,
}

impl<T: Clone + Debug> Cassette<T> {
    pub fn new() -> Self {
        Self {
            tape: Vector::new(),
            playback_index: 0,
        }
    }

    pub fn insert(&mut self, item: T){
        let playback_index = self.playback_index;

        if playback_index != self.tape.len() {
            self.tape = self.tape.slice(0..playback_index);
        }
        self.tape.push_back(item);
    }
    
    pub fn insert_and_play(&mut self, item: T) {
        let index = self.playback_index;
        self.insert(item);
        self.playback_index = index + 1
    }

    pub fn append(&mut self, other: Vector<T>) {
        let playback_index = self.playback_index;

        if playback_index != self.tape.len(){
            self.tape = self.tape.slice(0..playback_index)
        }
        self.tape.append(other);
    }

    pub fn append_and_play(&mut self, other: Vector<T>) {
        let index = self.playback_index;
        let diff = other.len();
        self.append(other);
        self.playback_index = index + diff;
    }

    pub fn undo(&mut self) -> Option<&T> {
        if self.playback_index != 0 {
            self.playback_index = self.playback_index - 1;
            return self.tape.get(self.playback_index);
        }
        None
    }
    
    pub fn redo(&mut self) -> Option<&T> {
        if self.playback_index != self.tape.len() {
            self.playback_index = self.playback_index + 1;
            return self.tape.get(self.playback_index-1);
        }
        None
    }
}