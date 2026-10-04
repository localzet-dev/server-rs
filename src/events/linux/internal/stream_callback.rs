pub struct StreamCallback {
    pub token: u64,
    pub callback: Box<dyn FnMut() + Send>,
}
