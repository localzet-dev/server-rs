use std::io;
use std::net::UdpSocket;
use std::sync::Arc;
use std::time::Duration;

use server_rs::{AsyncUdpConnection, Connection, UdpConnection};

#[test]
fn udp_connection_replies_to_a_remote_peer() {
    let server_socket = Arc::new(UdpSocket::bind("127.0.0.1:0").unwrap());
    let client = UdpSocket::bind("127.0.0.1:0").unwrap();
    client
        .set_read_timeout(Some(Duration::from_secs(1)))
        .unwrap();

    let mut connection =
        UdpConnection::new(7, Arc::clone(&server_socket), client.local_addr().unwrap());
    connection.send(b"pong").unwrap();

    let mut buffer = [0; 4];
    let (bytes_read, sender) = client.recv_from(&mut buffer).unwrap();
    assert_eq!(bytes_read, 4);
    assert_eq!(&buffer, b"pong");
    assert_eq!(sender, server_socket.local_addr().unwrap());
    assert_eq!(connection.id(), 7);
}

#[test]
fn async_udp_connection_connects_lazily_and_receives_data() -> io::Result<()> {
    let receiver = UdpSocket::bind("127.0.0.1:0")?;
    receiver.set_read_timeout(Some(Duration::from_secs(1)))?;
    let mut connection = AsyncUdpConnection::new(9, receiver.local_addr()?)?;
    assert!(!connection.is_connected());

    connection.send(b"ping")?;
    assert!(connection.is_connected());
    let mut request = [0; 4];
    let (bytes_read, sender) = receiver.recv_from(&mut request)?;
    assert_eq!(&request[..bytes_read], b"ping");

    receiver.send_to(b"pong", sender)?;
    assert_eq!(connection.receive()?, b"pong");
    connection.close()?;
    assert!(!connection.is_connected());
    Ok(())
}
