use std::io::{self, Read, Write};
use std::net::{Shutdown, SocketAddr, TcpStream, ToSocketAddrs};
use std::time::Duration;

use super::{Connection, ConnectionStatus};

pub struct AsyncTcpConnection {
    id: u64,
    remote_address: SocketAddr,
    stream: Option<TcpStream>,
    status: ConnectionStatus,
    connect_timeout: Duration,
}

impl AsyncTcpConnection {
    pub fn new(id: u64, address: impl ToSocketAddrs) -> io::Result<Self> {
        let remote_address = address
            .to_socket_addrs()?
            .next()
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "address resolved empty"))?;
        Ok(Self {
            id,
            remote_address,
            stream: None,
            status: ConnectionStatus::Initial,
            connect_timeout: Duration::from_secs(5),
        })
    }

    pub fn with_connect_timeout(mut self, timeout: Duration) -> Self {
        self.connect_timeout = timeout;
        self
    }
    pub fn status(&self) -> ConnectionStatus {
        self.status
    }

    pub fn connect(&mut self) -> io::Result<()> {
        if self.status == ConnectionStatus::Established {
            return Ok(());
        }
        self.status = ConnectionStatus::Connecting;
        match TcpStream::connect_timeout(&self.remote_address, self.connect_timeout) {
            Ok(stream) => {
                self.stream = Some(stream);
                self.status = ConnectionStatus::Established;
                Ok(())
            }
            Err(error) => {
                self.status = ConnectionStatus::Closed;
                Err(error)
            }
        }
    }

    pub fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        self.stream.as_mut().ok_or_else(not_connected)?.read(buffer)
    }
}

impl Connection for AsyncTcpConnection {
    fn id(&self) -> u64 {
        self.id
    }
    fn send(&mut self, data: &[u8]) -> io::Result<()> {
        if self.stream.is_none() {
            self.connect()?;
        }
        self.stream
            .as_mut()
            .ok_or_else(not_connected)?
            .write_all(data)
    }
    fn close(&mut self) -> io::Result<()> {
        if let Some(stream) = self.stream.take() {
            self.status = ConnectionStatus::Closing;
            let result = stream.shutdown(Shutdown::Both);
            self.status = ConnectionStatus::Closed;
            if let Err(error) = result
                && error.kind() != io::ErrorKind::NotConnected
            {
                return Err(error);
            }
        } else {
            self.status = ConnectionStatus::Closed;
        }
        Ok(())
    }
    fn local_address(&self) -> io::Result<SocketAddr> {
        self.stream.as_ref().ok_or_else(not_connected)?.local_addr()
    }
    fn remote_address(&self) -> io::Result<SocketAddr> {
        Ok(self.remote_address)
    }
}

fn not_connected() -> io::Error {
    io::Error::new(io::ErrorKind::NotConnected, "TCP socket is not connected")
}
