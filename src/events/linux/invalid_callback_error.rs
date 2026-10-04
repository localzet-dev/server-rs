use std::fmt;
#[derive(Debug)]
pub struct InvalidCallbackError(pub String);
impl fmt::Display for InvalidCallbackError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for InvalidCallbackError {}
