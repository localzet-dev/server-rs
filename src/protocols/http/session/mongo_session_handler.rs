use super::SessionHandler;
use std::io;
use std::time::Duration;

#[derive(Clone, Debug)]
pub struct MongoSessionHandler {
    uri: String,
    database: String,
    collection: String,
}

impl MongoSessionHandler {
    pub fn new(
        uri: impl Into<String>,
        database: impl Into<String>,
        collection: impl Into<String>,
    ) -> Self {
        Self {
            uri: uri.into(),
            database: database.into(),
            collection: collection.into(),
        }
    }
    pub fn uri(&self) -> &str {
        &self.uri
    }
    pub fn database(&self) -> &str {
        &self.database
    }
    pub fn collection(&self) -> &str {
        &self.collection
    }
}

impl SessionHandler for MongoSessionHandler {
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
        "MongoDB session storage requires an application MongoDB client adapter",
    )
}
