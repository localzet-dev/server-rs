use std::collections::HashMap;
use std::sync::Mutex;
use std::thread::ThreadId;
pub struct FiberLocal<T> {
    values: Mutex<HashMap<ThreadId, T>>,
}
impl<T> Default for FiberLocal<T> {
    fn default() -> Self {
        Self {
            values: Mutex::new(HashMap::new()),
        }
    }
}
impl<T: Clone> FiberLocal<T> {
    pub fn set(&self, value: T) {
        self.values
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .insert(std::thread::current().id(), value);
    }
    pub fn get(&self) -> Option<T> {
        self.values
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .get(&std::thread::current().id())
            .cloned()
    }
    pub fn unset(&self) {
        self.values
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .remove(&std::thread::current().id());
    }
}
