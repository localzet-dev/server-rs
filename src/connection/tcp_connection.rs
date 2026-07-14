use std::io::{self, Read, Write};
use std::net::{Shutdown, SocketAddr, TcpStream};
use std::sync::Arc;
use std::time::Duration;

use super::{Connection, ConnectionStatistics};

pub struct TcpConnection {
    id: u64,
    stream: TcpStream,
    statistics: Arc<ConnectionStatistics>,
    closed: bool,
}

impl TcpConnection {
    pub(crate) fn new(
        id: u64,
        stream: TcpStream,
        statistics: Arc<ConnectionStatistics>,
        read_timeout: Duration,
    ) -> io::Result<Self> {
        stream.set_read_timeout(Some(read_timeout))?;
        statistics.connection_opened();
        Ok(Self {
            id,
            stream,
            statistics,
            closed: false,
        })
    }

    pub(crate) fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        let bytes_read = self.stream.read(buffer)?;
        if bytes_read > 0 {
            self.statistics.request_received(bytes_read);
        }
        Ok(bytes_read)
    }
}

impl Connection for TcpConnection {
    fn id(&self) -> u64 {
        self.id
    }

    fn send(&mut self, data: &[u8]) -> io::Result<()> {
        match self.stream.write_all(data) {
            Ok(()) => {
                self.statistics.data_sent(data.len());
                Ok(())
            }
            Err(error) => {
                self.statistics.send_failed();
                Err(error)
            }
        }
    }

    fn close(&mut self) -> io::Result<()> {
        if self.closed {
            return Ok(());
        }

        self.closed = true;
        match self.stream.shutdown(Shutdown::Both) {
            Err(error) if error.kind() == io::ErrorKind::NotConnected => Ok(()),
            result => result,
        }
    }

    fn local_address(&self) -> io::Result<SocketAddr> {
        self.stream.local_addr()
    }

    fn remote_address(&self) -> io::Result<SocketAddr> {
        self.stream.peer_addr()
    }
}

impl Drop for TcpConnection {
    fn drop(&mut self) {
        let _ = self.close();
        self.statistics.connection_closed();
    }
}
