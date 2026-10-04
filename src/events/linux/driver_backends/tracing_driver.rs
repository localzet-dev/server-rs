use crate::TimerId;
use crate::events::{BoxedCallback, EventLoop};
use std::io;
use std::sync::Arc;
use std::time::Duration;
pub struct TracingDriver {
    inner: Arc<dyn EventLoop>,
}
impl TracingDriver {
    pub fn new(inner: Arc<dyn EventLoop>) -> Self {
        Self { inner }
    }
}
impl EventLoop for TracingDriver {
    fn run(&self) {
        self.inner.run()
    }
    fn stop(&self) {
        self.inner.stop()
    }
    fn defer(&self, c: Box<dyn FnOnce() + Send + 'static>) {
        self.inner.defer(c)
    }
    fn delay(&self, d: Duration, c: Box<dyn FnOnce() + Send + 'static>) -> TimerId {
        self.inner.delay(d, c)
    }
    fn repeat(&self, d: Duration, c: BoxedCallback) -> io::Result<TimerId> {
        self.inner.repeat(d, c)
    }
    fn off_timer(&self, id: TimerId) -> bool {
        self.inner.off_timer(id)
    }
    fn delete_all_timers(&self) {
        self.inner.delete_all_timers()
    }
}
