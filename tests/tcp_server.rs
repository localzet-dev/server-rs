use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use server_rs::{Connection, Server, ServerConfig, ServerHandler, ServerStatus};

struct EchoHandler {
    started: mpsc::Sender<()>,
}

impl ServerHandler for EchoHandler {
    fn on_server_start(&self, _config: &ServerConfig) {
        self.started.send(()).unwrap();
    }

    fn on_message(&self, connection: &mut dyn Connection, data: &[u8]) -> std::io::Result<()> {
        connection.send(data)
    }
}

#[test]
fn echoes_data_and_collects_statistics() {
    let address = available_address();
    let (started_tx, started_rx) = mpsc::channel();
    let mut server = Server::new(
        ServerConfig::new(address).with_name("echo"),
        EchoHandler {
            started: started_tx,
        },
    );
    let handle = server.handle();

    let server_thread = thread::spawn(move || {
        server.run().unwrap();
        server.statistics()
    });
    started_rx.recv_timeout(Duration::from_secs(1)).unwrap();

    let mut client = TcpStream::connect(address).unwrap();
    client.write_all(b"hello").unwrap();
    let mut response = [0; 5];
    client.read_exact(&mut response).unwrap();
    assert_eq!(&response, b"hello");
    drop(client);

    wait_until(|| handle.status() == ServerStatus::Running);
    handle.stop();
    let statistics = server_thread.join().unwrap();

    assert_eq!(statistics.connection_count, 0);
    assert_eq!(statistics.total_request, 1);
    assert_eq!(statistics.read_bytes, 5);
    assert_eq!(statistics.written_bytes, 5);
    assert_eq!(statistics.send_fail, 0);
    assert_eq!(handle.status(), ServerStatus::Shutdown);
}

#[test]
fn stops_while_a_client_is_connected() {
    let address = available_address();
    let (started_tx, started_rx) = mpsc::channel();
    let mut server = Server::new(
        ServerConfig::new(address),
        EchoHandler {
            started: started_tx,
        },
    );
    let handle = server.handle();
    let server_thread = thread::spawn(move || server.run().unwrap());
    started_rx.recv_timeout(Duration::from_secs(1)).unwrap();

    let _client = TcpStream::connect(address).unwrap();
    handle.stop();

    server_thread.join().unwrap();
    assert_eq!(handle.status(), ServerStatus::Shutdown);
}

fn available_address() -> SocketAddr {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    drop(listener);
    address
}

fn wait_until(predicate: impl Fn() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(1);
    while !predicate() {
        assert!(Instant::now() < deadline, "condition timed out");
        thread::sleep(Duration::from_millis(10));
    }
}
