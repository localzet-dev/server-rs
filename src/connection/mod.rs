mod async_udp_connection;
mod base;
mod tcp_connection;
mod udp_connection;

pub use async_udp_connection::AsyncUdpConnection;
pub use base::{Connection, ConnectionErrorCode, ConnectionStatistics, StatisticsSnapshot};
pub use tcp_connection::{ConnectionStatus, TcpConnection};
pub use udp_connection::{MAX_UDP_PACKAGE_SIZE, UdpConnection};
