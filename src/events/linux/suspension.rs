use std::sync::{Condvar, Mutex};
pub struct Suspension<T> {
    value: Mutex<Option<T>>,
    resumed: Condvar,
}
impl<T> Default for Suspension<T> {
    fn default() -> Self {
        Self {
            value: Mutex::new(None),
            resumed: Condvar::new(),
        }
    }
}
impl<T> Suspension<T> {
    pub fn resume(&self, value: T) {
        *self.value.lock().unwrap_or_else(|p| p.into_inner()) = Some(value);
        self.resumed.notify_one();
    }
    pub fn suspend(&self) -> T {
        let mut value = self.value.lock().unwrap_or_else(|p| p.into_inner());
        while value.is_none() {
            value = self.resumed.wait(value).unwrap_or_else(|p| p.into_inner());
        }
        value.take().unwrap()
    }
}
