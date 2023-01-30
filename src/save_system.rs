use druid::{im::Vector, Data, Lens};

#[derive(Clone, PartialEq, Data, Lens)]
pub struct SaveSystemData<T: Clone>{
    save_stack: Vector<T>,
    restore_stack: Vector<T>,
    playback_index: usize,
}

impl<T: Clone> SaveSystemData<T> {
    pub fn new() -> Self {
        Self {
            save_stack: Vector::new(),
            restore_stack: Vector::new(),
            playback_index: 0,
        }
    }

    pub fn submit_play(&mut self, item: T) {
        self.submit(item);
        self.playback_index += 1;
    }

    pub fn submit(&mut self, item: T){
        if self.playback_index != self.save_stack.len() {
            self.save_stack = self.save_stack.slice(0..self.playback_index)
        }
        self.save_stack.push_back(item);
    }

    pub fn append_play(&mut self, other: Vector<T>){
        let diff = other.len();
        self.append(other);
        self.playback_index += diff;
    }

    pub fn append(&mut self, other: Vector<T>){
        if self.playback_index != self.save_stack.len(){
            self.save_stack = self.save_stack.slice(0..self.playback_index)
        }
        self.save_stack.append(other);   
    }
    
    pub fn undo(&mut self){
        if self.playback_index != 0 {
            self.playback_index -= 1;
        }
    }
    
    pub fn redo(&mut self){
        if self.playback_index != self.save_stack.len() {
            self.playback_index += 1;
        }
    }
    
    fn save(&mut self){
        todo!()
    }

    fn restore(&mut self){
        todo!()
    }

    pub fn get_last_item(&self) -> Option<&T>{
        self.save_stack.last()
    }

    pub fn get_playback_index(&self) -> usize{
        self.playback_index
    }

    pub fn get(&self, index: usize) -> Option<&T>{
        self.save_stack.get(index)
    }

}