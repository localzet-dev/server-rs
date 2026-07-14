use std::io;

use server_rs::{Connection, Server, ServerConfig, ServerHandler};

struct EchoHandler;

impl ServerHandler for EchoHandler {
    fn on_message(&self, connection: &mut dyn Connection, data: &[u8]) -> io::Result<()> {
        connection.send(data)
    }
}

fn main() -> io::Result<()> {
    let address = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "127.0.0.1:8080".to_owned())
        .parse()
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidInput, error))?;

    let mut server = Server::new(ServerConfig::new(address), EchoHandler);
    println!("{} listening on {}", server.config().name(), address);
    server.run()
}
