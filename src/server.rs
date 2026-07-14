use std::io;
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU8, AtomicU64, Ordering};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use crate::{ConnectionStatistics, ServerHandler, StatisticsSnapshot, TcpConnection};

pub const VERSION: &str = "0.1.0";
const ACCEPT_POLL_INTERVAL: Duration = Duration::from_millis(10);
const CONNECTION_POLL_INTERVAL: Duration = Duration::from_millis(100);

#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ServerStatus {
    Initial = 0,
    Starting = 1,
    Running = 2,
    Shutdown = 4,
}

impl ServerStatus {
    fn from_u8(value: u8) -> Self {
        match value {
            1 => Self::Starting,
            2 => Self::Running,
            4 => Self::Shutdown,
            _ => Self::Initial,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ServerConfig {
    name: String,
    address: SocketAddr,
    read_buffer_size: usize,
}

impl ServerConfig {
    pub fn new(address: SocketAddr) -> Self {
        Self {
            name: "none".to_owned(),
            address,
            read_buffer_size: 87_380,
        }
    }

    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    pub fn with_read_buffer_size(mut self, size: usize) -> Self {
        assert!(size > 0, "read buffer size must be greater than zero");
        self.read_buffer_size = size;
        self
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn address(&self) -> SocketAddr {
        self.address
    }

    pub fn read_buffer_size(&self) -> usize {
        self.read_buffer_size
    }
}

#[derive(Clone)]
pub struct ServerHandle {
    stopping: Arc<AtomicBool>,
    paused: Arc<AtomicBool>,
    status: Arc<AtomicU8>,
}

impl ServerHandle {
    pub fn stop(&self) {
        self.stopping.store(true, Ordering::Release);
    }

    pub fn pause_accept(&self) {
        self.paused.store(true, Ordering::Release);
    }

    pub fn resume_accept(&self) {
        self.paused.store(false, Ordering::Release);
    }

    pub fn status(&self) -> ServerStatus {
        ServerStatus::from_u8(self.status.load(Ordering::Acquire))
    }
}

pub struct Server<H: ServerHandler> {
    config: ServerConfig,
    handler: Arc<H>,
    handle: ServerHandle,
    statistics: Arc<ConnectionStatistics>,
    next_connection_id: AtomicU64,
}

impl<H: ServerHandler> Server<H> {
    pub fn new(config: ServerConfig, handler: H) -> Self {
        Self {
            config,
            handler: Arc::new(handler),
            handle: ServerHandle {
                stopping: Arc::new(AtomicBool::new(false)),
                paused: Arc::new(AtomicBool::new(false)),
                status: Arc::new(AtomicU8::new(ServerStatus::Initial as u8)),
            },
            statistics: Arc::new(ConnectionStatistics::default()),
            next_connection_id: AtomicU64::new(1),
        }
    }

    pub fn config(&self) -> &ServerConfig {
        &self.config
    }

    pub fn handle(&self) -> ServerHandle {
        self.handle.clone()
    }

    pub fn status(&self) -> ServerStatus {
        self.handle.status()
    }

    pub fn statistics(&self) -> StatisticsSnapshot {
        self.statistics.snapshot()
    }

    pub fn run(&mut self) -> io::Result<()> {
        self.set_status(ServerStatus::Starting);
        self.handle.stopping.store(false, Ordering::Release);

        let listener = match TcpListener::bind(self.config.address) {
            Ok(listener) => listener,
            Err(error) => {
                self.set_status(ServerStatus::Shutdown);
                self.handler.on_error(None, &error);
                return Err(error);
            }
        };
        if let Err(error) = listener.set_nonblocking(true) {
            self.set_status(ServerStatus::Shutdown);
            self.handler.on_error(None, &error);
            return Err(error);
        }

        self.set_status(ServerStatus::Running);
        self.handler.on_server_start(&self.config);

        let mut workers = Vec::new();
        while !self.handle.stopping.load(Ordering::Acquire) {
            if self.handle.paused.load(Ordering::Acquire) {
                thread::sleep(ACCEPT_POLL_INTERVAL);
                continue;
            }

            match listener.accept() {
                Ok((stream, _)) => self.spawn_connection(stream, &mut workers),
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                    thread::sleep(ACCEPT_POLL_INTERVAL);
                }
                Err(error) => self.handler.on_error(None, &error),
            }
            workers.retain(|worker| !worker.is_finished());
        }

        for worker in workers {
            let _ = worker.join();
        }
        self.set_status(ServerStatus::Shutdown);
        self.handler.on_server_stop(self.statistics());
        Ok(())
    }

    fn spawn_connection(&self, stream: TcpStream, workers: &mut Vec<JoinHandle<()>>) {
        let id = self.next_connection_id.fetch_add(1, Ordering::Relaxed);
        let statistics = Arc::clone(&self.statistics);
        let handler = Arc::clone(&self.handler);
        let stopping = Arc::clone(&self.handle.stopping);
        let buffer_size = self.config.read_buffer_size;

        workers.push(thread::spawn(move || {
            let mut connection =
                match TcpConnection::new(id, stream, statistics, CONNECTION_POLL_INTERVAL) {
                    Ok(connection) => connection,
                    Err(error) => {
                        handler.on_error(None, &error);
                        return;
                    }
                };
            handler.on_connect(&mut connection);
            let mut buffer = vec![0; buffer_size];

            while !stopping.load(Ordering::Acquire) {
                match connection.read(&mut buffer) {
                    Ok(0) => break,
                    Ok(bytes_read) => {
                        if let Err(error) =
                            handler.on_message(&mut connection, &buffer[..bytes_read])
                        {
                            handler.on_error(Some(&connection), &error);
                            break;
                        }
                    }
                    Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                    Err(error)
                        if matches!(
                            error.kind(),
                            io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut
                        ) =>
                    {
                        continue;
                    }
                    Err(error) => {
                        handler.on_error(Some(&connection), &error);
                        break;
                    }
                }
            }

            handler.on_close(&connection);
        }));
    }

    fn set_status(&self, status: ServerStatus) {
        self.handle.status.store(status as u8, Ordering::Release);
    }
}
