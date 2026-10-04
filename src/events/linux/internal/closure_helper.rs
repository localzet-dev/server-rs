pub fn invoke(callback: impl FnOnce()) {
    callback();
}
