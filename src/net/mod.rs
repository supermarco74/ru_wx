//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Networking (`wxSocket`, `wxSocketServer`, `wxWebRequest`).

mod connection;
mod ipc_client;
mod ipc_server;
mod ftp;
mod http;
mod protocol;
mod socket_event;
mod socket_server;
mod url;

pub use connection::IpcConnection;
pub use ipc_client::IpcClient;
pub use ipc_server::IpcServer;
pub use ftp::FtpClient;
pub use http::HttpClient;
pub use protocol::Protocol;
pub use socket_event::{SocketEvent, SocketEventKind};
pub use socket_server::SocketServer;
pub use url::Url;

use std::cell::RefCell;
use std::io::{self, Read, Write};
use std::net::TcpStream;

/// Replaceable socket event callback slot.
type SocketEventHandler = RefCell<Option<Box<dyn FnMut(&SocketEvent)>>>;

/// TCP socket (`wxSocket`).
pub struct Socket {
    host: String,
    port: u16,
    connected: bool,
    stream: RefCell<Option<TcpStream>>,
    on_event: SocketEventHandler,
}

impl Default for Socket {
    fn default() -> Self {
        Self {
            host: String::new(),
            port: 0,
            connected: false,
            stream: RefCell::new(None),
            on_event: RefCell::new(None),
        }
    }
}

impl std::fmt::Debug for Socket {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Socket")
            .field("host", &self.host)
            .field("port", &self.port)
            .field("connected", &self.connected)
            .finish_non_exhaustive()
    }
}

impl Socket {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn connect(&mut self, host: &str, port: u16) -> io::Result<()> {
        let addr = format!("{host}:{port}");
        let stream = TcpStream::connect(addr)?;
        stream.set_read_timeout(None).ok();
        stream.set_write_timeout(None).ok();
        self.host = host.to_string();
        self.port = port;
        self.connected = true;
        *self.stream.borrow_mut() = Some(stream);
        Ok(())
    }

    pub fn disconnect(&mut self) {
        *self.stream.borrow_mut() = None;
        self.connected = false;
    }

    pub fn is_connected(&self) -> bool {
        self.connected && self.stream.borrow().is_some()
    }

    pub fn endpoint(&self) -> (&str, u16) {
        (&self.host, self.port)
    }

    pub fn write_all(&self, data: &[u8]) -> io::Result<()> {
        let mut guard = self.stream.borrow_mut();
        let stream = guard
            .as_mut()
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotConnected, "socket not connected"))?;
        stream.write_all(data)
    }

    pub fn read(&self, buf: &mut [u8]) -> io::Result<usize> {
        let mut guard = self.stream.borrow_mut();
        let stream = guard
            .as_mut()
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotConnected, "socket not connected"))?;
        stream.read(buf)
    }

    pub fn on_socket_event<F: FnMut(&SocketEvent) + 'static>(&self, f: F) {
        *self.on_event.borrow_mut() = Some(Box::new(f));
    }

    pub fn notify_input(&self, bytes_available: usize) {
        if let Some(ref mut cb) = *self.on_event.borrow_mut() {
            cb(&SocketEvent::new(SocketEventKind::Input, bytes_available));
        }
    }
}

/// HTTP request helper (`wxWebRequest`).
#[derive(Debug, Clone)]
pub struct WebRequest {
    url: String,
    method: String,
    body: Vec<u8>,
}

impl WebRequest {
    pub fn get(url: &str) -> Self {
        Self {
            url: url.to_string(),
            method: "GET".into(),
            body: Vec::new(),
        }
    }

    pub fn post(url: &str) -> Self {
        Self {
            url: url.to_string(),
            method: "POST".into(),
            body: Vec::new(),
        }
    }

    pub fn with_body(mut self, body: impl AsRef<[u8]>) -> Self {
        self.body = body.as_ref().to_vec();
        self
    }

    pub fn url(&self) -> &str {
        &self.url
    }

    pub fn execute(&self) -> io::Result<Vec<u8>> {
        let client = HttpClient::new();
        match self.method.as_str() {
            "GET" => client.get(&self.url),
            "POST" => client.post(&self.url, &self.body),
            other => Err(io::Error::new(
                io::ErrorKind::Unsupported,
                format!("unsupported HTTP method: {other}"),
            )),
        }
    }
}
