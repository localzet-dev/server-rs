use super::{BoxedCallback, EventLoop};
use crate::{Timer, TimerId};
use std::collections::VecDeque;
use std::io;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Condvar, Mutex};
use std::time::Duration;

pub struct Event {
    timer: Timer,
    deferred: Mutex<VecDeque<Box<dyn FnOnce() + Send>>>,
    changed: Condvar,
    stopped: AtomicBool,
}

impl Default for Event {
    fn default() -> Self {
        Self::new()
    }
}

impl Event {
    pub fn new() -> Self {
        Self {
            timer: Timer::new(),
            deferred: Mutex::new(VecDeque::new()),
            changed: Condvar::new(),
            stopped: AtomicBool::new(false),
        }
    }
}

impl EventLoop for Event {
    fn run(&self) {
        self.stopped.store(false, Ordering::Release);
        while !self.stopped.load(Ordering::Acquire) {
            let callback = {
                let mut queue = self.deferred.lock().unwrap_or_else(|p| p.into_inner());
                while queue.is_empty() && !self.stopped.load(Ordering::Acquire) {
                    queue = self.changed.wait(queue).unwrap_or_else(|p| p.into_inner());
                }
                queue.pop_front()
            };
            if let Some(callback) = callback {
                callback();
            }
        }
    }
    fn stop(&self) {
        self.stopped.store(true, Ordering::Release);
        self.changed.notify_all();
    }
    fn defer(&self, callback: Box<dyn FnOnce() + Send + 'static>) {
        self.deferred
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .push_back(callback);
        self.changed.notify_one();
    }
    fn delay(&self, delay: Duration, callback: Box<dyn FnOnce() + Send + 'static>) -> TimerId {
        self.timer.delay(delay, callback)
    }
    fn repeat(&self, interval: Duration, callback: BoxedCallback) -> io::Result<TimerId> {
        self.timer.repeat(interval, callback)
    }
    fn off_timer(&self, id: TimerId) -> bool {
        self.timer.delete(id)
    }
    fn delete_all_timers(&self) {
        self.timer.delete_all();
    }
}
