use std::io;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, ToSocketAddrs, UdpSocket};

use super::{Connection, MAX_UDP_PACKAGE_SIZE};

pub struct AsyncUdpConnection {
    id: u64,
    remote_address: SocketAddr,
    socket: Option<UdpSocket>,
}

impl AsyncUdpConnection {
    pub fn new(id: u64, remote_address: impl ToSocketAddrs) -> io::Result<Self> {
        let remote_address = remote_address
            .to_socket_addrs()?
            .next()
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "address resolved empty"))?;
        Ok(Self {
            id,
            remote_address,
            socket: None,
        })
    }

    pub fn connect(&mut self) -> io::Result<()> {
        if self.socket.is_some() {
            return Ok(());
        }

        let bind_address = match self.remote_address.ip() {
            IpAddr::V4(_) => SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0),
            IpAddr::V6(_) => SocketAddr::new(IpAddr::V6(Ipv6Addr::UNSPECIFIED), 0),
        };
        let socket = UdpSocket::bind(bind_address)?;
        socket.connect(self.remote_address)?;
        self.socket = Some(socket);
        Ok(())
    }

    pub fn is_connected(&self) -> bool {
        self.socket.is_some()
    }

    pub fn receive(&self) -> io::Result<Vec<u8>> {
        let socket = self.socket.as_ref().ok_or_else(not_connected)?;
        let mut buffer = vec![0; MAX_UDP_PACKAGE_SIZE];
        let bytes_read = socket.recv(&mut buffer)?;
        buffer.truncate(bytes_read);
        Ok(buffer)
    }

    pub fn set_nonblocking(&self, nonblocking: bool) -> io::Result<()> {
        self.socket
            .as_ref()
            .ok_or_else(not_connected)?
            .set_nonblocking(nonblocking)
    }
}

impl Connection for AsyncUdpConnection {
    fn id(&self) -> u64 {
        self.id
    }

    fn send(&mut self, data: &[u8]) -> io::Result<()> {
        if data.len() > MAX_UDP_PACKAGE_SIZE {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "UDP datagram exceeds the maximum package size",
            ));
        }
        self.connect()?;
        let written = self.socket.as_ref().ok_or_else(not_connected)?.send(data)?;
        if written != data.len() {
            return Err(io::Error::new(
                io::ErrorKind::WriteZero,
                "UDP datagram was not sent completely",
            ));
        }
        Ok(())
    }

    fn close(&mut self) -> io::Result<()> {
        self.socket = None;
        Ok(())
    }

    fn local_address(&self) -> io::Result<SocketAddr> {
        self.socket.as_ref().ok_or_else(not_connected)?.local_addr()
    }

    fn remote_address(&self) -> io::Result<SocketAddr> {
        Ok(self.remote_address)
    }
}

fn not_connected() -> io::Error {
    io::Error::new(io::ErrorKind::NotConnected, "UDP socket is not connected")
}
