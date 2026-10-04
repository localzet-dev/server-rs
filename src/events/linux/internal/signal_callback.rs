pub struct SignalCallback {
    pub signal: i32,
    pub callback: Box<dyn FnMut(i32) + Send>,
}
