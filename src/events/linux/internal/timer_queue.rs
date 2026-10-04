use std::collections::BTreeSet;
use std::time::Instant;
#[derive(Default)]
pub struct TimerQueue(BTreeSet<(Instant, u64)>);
impl TimerQueue {
    pub fn insert(&mut self, at: Instant, id: u64) {
        self.0.insert((at, id));
    }
    pub fn remove(&mut self, at: Instant, id: u64) -> bool {
        self.0.remove(&(at, id))
    }
    pub fn first(&self) -> Option<(Instant, u64)> {
        self.0.first().copied()
    }
}
