use std::io;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicU64, Ordering};

#[repr(i32)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConnectionErrorCode {
    ConnectFail = 1,
    SendFail = 2,
}

#[derive(Debug, Default)]
pub struct ConnectionStatistics {
    connection_count: AtomicU64,
    total_request: AtomicU64,
    read_bytes: AtomicU64,
    written_bytes: AtomicU64,
    send_fail: AtomicU64,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct StatisticsSnapshot {
    pub connection_count: u64,
    pub total_request: u64,
    pub read_bytes: u64,
    pub written_bytes: u64,
    pub send_fail: u64,
}

impl ConnectionStatistics {
    pub fn snapshot(&self) -> StatisticsSnapshot {
        StatisticsSnapshot {
            connection_count: self.connection_count.load(Ordering::Relaxed),
            total_request: self.total_request.load(Ordering::Relaxed),
            read_bytes: self.read_bytes.load(Ordering::Relaxed),
            written_bytes: self.written_bytes.load(Ordering::Relaxed),
            send_fail: self.send_fail.load(Ordering::Relaxed),
        }
    }

    pub(crate) fn connection_opened(&self) {
        self.connection_count.fetch_add(1, Ordering::Relaxed);
    }

    pub(crate) fn connection_closed(&self) {
        self.connection_count.fetch_sub(1, Ordering::Relaxed);
    }

    pub(crate) fn request_received(&self, bytes: usize) {
        self.total_request.fetch_add(1, Ordering::Relaxed);
        self.read_bytes.fetch_add(bytes as u64, Ordering::Relaxed);
    }

    pub(crate) fn data_sent(&self, bytes: usize) {
        self.written_bytes
            .fetch_add(bytes as u64, Ordering::Relaxed);
    }

    pub(crate) fn send_failed(&self) {
        self.send_fail.fetch_add(1, Ordering::Relaxed);
    }
}

pub trait Connection {
    fn id(&self) -> u64;
    fn send(&mut self, data: &[u8]) -> io::Result<()>;
    fn close(&mut self) -> io::Result<()>;
    fn local_address(&self) -> io::Result<SocketAddr>;
    fn remote_address(&self) -> io::Result<SocketAddr>;

    fn is_ipv4(&self) -> bool {
        self.remote_address().is_ok_and(|address| address.is_ipv4())
    }

    fn is_ipv6(&self) -> bool {
        self.remote_address().is_ok_and(|address| address.is_ipv6())
    }
}
