use std::fmt;
#[derive(Debug)]
pub struct UnsupportedFeatureError(pub &'static str);
impl fmt::Display for UnsupportedFeatureError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0)
    }
}
impl std::error::Error for UnsupportedFeatureError {}
