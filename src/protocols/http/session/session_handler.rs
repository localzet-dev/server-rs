use std::io;
use std::time::Duration;

pub trait SessionHandler: Send + Sync {
    fn open(&self, _name: &str) -> io::Result<()> {
        Ok(())
    }

    fn close(&self) -> io::Result<()> {
        Ok(())
    }

    fn read(&self, session_id: &str, lifetime: Duration) -> io::Result<Option<Vec<u8>>>;
    fn write(&self, session_id: &str, session_data: &[u8]) -> io::Result<()>;
    fn update_timestamp(&self, session_id: &str) -> io::Result<bool>;
    fn destroy(&self, session_id: &str) -> io::Result<bool>;
    fn gc(&self, max_lifetime: Duration) -> io::Result<usize>;
}
