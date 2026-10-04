use std::io;
use std::time::Duration;

use super::SessionHandler;

#[derive(Clone, Debug)]
pub struct RedisSessionHandler {
    endpoint: String,
    prefix: String,
}

impl RedisSessionHandler {
    pub fn new(endpoint: impl Into<String>, prefix: impl Into<String>) -> Self {
        Self {
            endpoint: endpoint.into(),
            prefix: prefix.into(),
        }
    }

    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }
    pub fn prefix(&self) -> &str {
        &self.prefix
    }
}

impl SessionHandler for RedisSessionHandler {
    fn read(&self, _: &str, _: Duration) -> io::Result<Option<Vec<u8>>> {
        Err(unavailable())
    }
    fn write(&self, _: &str, _: &[u8]) -> io::Result<()> {
        Err(unavailable())
    }
    fn update_timestamp(&self, _: &str) -> io::Result<bool> {
        Err(unavailable())
    }
    fn destroy(&self, _: &str) -> io::Result<bool> {
        Err(unavailable())
    }
    fn gc(&self, _: Duration) -> io::Result<usize> {
        Ok(0)
    }
}

fn unavailable() -> io::Error {
    io::Error::new(
        io::ErrorKind::Unsupported,
        "Redis session storage requires an application Redis client adapter",
    )
}
