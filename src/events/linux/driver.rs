pub trait Driver: crate::events::EventLoop {}
impl<T: crate::events::EventLoop> Driver for T {}
