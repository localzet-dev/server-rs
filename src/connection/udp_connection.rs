use std::io;
use std::net::{SocketAddr, UdpSocket};
use std::sync::Arc;

use super::Connection;

pub const MAX_UDP_PACKAGE_SIZE: usize = 65_535;

pub struct UdpConnection {
    id: u64,
    socket: Arc<UdpSocket>,
    remote_address: SocketAddr,
    closed: bool,
}

impl UdpConnection {
    pub fn new(id: u64, socket: Arc<UdpSocket>, remote_address: SocketAddr) -> Self {
        Self {
            id,
            socket,
            remote_address,
            closed: false,
        }
    }

    pub fn socket(&self) -> &UdpSocket {
        &self.socket
    }
}

impl Connection for UdpConnection {
    fn id(&self) -> u64 {
        self.id
    }

    fn send(&mut self, data: &[u8]) -> io::Result<()> {
        if self.closed {
            return Err(io::Error::new(
                io::ErrorKind::BrokenPipe,
                "UDP connection is closed",
            ));
        }
        if data.len() > MAX_UDP_PACKAGE_SIZE {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "UDP datagram exceeds the maximum package size",
            ));
        }

        let written = self.socket.send_to(data, self.remote_address)?;
        if written != data.len() {
            return Err(io::Error::new(
                io::ErrorKind::WriteZero,
                "UDP datagram was not sent completely",
            ));
        }
        Ok(())
    }

    fn close(&mut self) -> io::Result<()> {
        self.closed = true;
        Ok(())
    }

    fn local_address(&self) -> io::Result<SocketAddr> {
        self.socket.local_addr()
    }

    fn remote_address(&self) -> io::Result<SocketAddr> {
        Ok(self.remote_address)
    }
}
