use crate::events::{Event, EventLoop};
use std::sync::Arc;
pub struct DriverFactory;
impl DriverFactory {
    pub fn create() -> Arc<dyn EventLoop> {
        Arc::new(Event::new())
    }
}
