mod ev;
mod event;
mod event_interface;
pub mod linux;
mod swoole;
mod swow;
mod windows;

pub use ev::Ev;
pub use event::Event;
pub use event_interface::{BoxedCallback, EventLoop};
pub use linux::Linux;
pub use swoole::Swoole;
pub use swow::Swow;
pub use windows::Windows;
