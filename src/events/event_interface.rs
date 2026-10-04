use crate::TimerId;
use std::io;
use std::time::Duration;

pub type BoxedCallback = Box<dyn FnMut() + Send + 'static>;

pub trait EventLoop: Send + Sync {
    fn run(&self);
    fn stop(&self);
    fn defer(&self, callback: Box<dyn FnOnce() + Send + 'static>);
    fn delay(&self, delay: Duration, callback: Box<dyn FnOnce() + Send + 'static>) -> TimerId;
    fn repeat(&self, interval: Duration, callback: BoxedCallback) -> io::Result<TimerId>;
    fn off_timer(&self, id: TimerId) -> bool;
    fn delete_all_timers(&self);
}
