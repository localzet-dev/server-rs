use std::io;

use crate::{Connection, ServerConfig, StatisticsSnapshot};

pub trait ServerHandler: Send + Sync + 'static {
    fn on_server_start(&self, _config: &ServerConfig) {}

    fn on_server_stop(&self, _statistics: StatisticsSnapshot) {}

    fn on_connect(&self, _connection: &mut dyn Connection) {}

    fn on_message(&self, _connection: &mut dyn Connection, _data: &[u8]) -> io::Result<()> {
        Ok(())
    }

    fn on_close(&self, _connection: &dyn Connection) {}

    fn on_error(&self, _connection: Option<&dyn Connection>, _error: &io::Error) {}
}
