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
pub struct SaveSystemData<T: Clone + Debug> {
    pub save_stack: Vector<T>,
    pub playback_index: usize,
}

impl<T: Clone + Debug> SaveSystemData<T> {
    pub fn new() -> Self {
        Self {
            save_stack: Vector::new(),
            playback_index: 0,
        }
    }

    pub fn submit(&mut self, item: T){
        let playback_index = self.playback_index;

        if playback_index != self.save_stack.len() {
            self.save_stack = self.save_stack.slice(0..playback_index);
        }
        self.save_stack.push_back(item);
    }
    
    pub fn submit_and_process(&mut self, item: T) {
        let index = self.playback_index;
        self.submit(item);
        self.playback_index = index + 1
    }

    pub fn append(&mut self, other: Vector<T>) {
        let playback_index = self.playback_index;

        if playback_index != self.save_stack.len(){
            self.save_stack = self.save_stack.slice(0..playback_index)
        }
        self.save_stack.append(other);
    }

    pub fn append_and_process(&mut self, other: Vector<T>) {
        let index = self.playback_index;
        let diff = other.len();

        self.append(other);
        self.playback_index = index + diff;
    }

    pub fn undo(&mut self){
        let playback_index = self.playback_index;

        if playback_index != 0 {
            self.playback_index = playback_index - 1;
        }
    }
    
    pub fn redo(&mut self){
        let playback_index = self.playback_index;

        if playback_index != self.save_stack.len() {
            self.playback_index = playback_index + 1;
        }
    }
    
    #[allow(dead_code)]
    fn save(&mut self){
        todo!()
    }

    #[allow(dead_code)]
    fn restore(&mut self){
        todo!()
    }

}