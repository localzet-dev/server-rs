#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CallbackType {
    Defer,
    Delay,
    Repeat,
    Readable,
    Writable,
    Signal,
}
