pub struct TimerCallback {
    pub id: u64,
    pub callback: Box<dyn FnMut() + Send>,
}
