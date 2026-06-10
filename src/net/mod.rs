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

use std::io;

/// Replaceable socket event callback slot.
type SocketEventHandler = std::cell::RefCell<Option<Box<dyn FnMut(&SocketEvent)>>>;

/// TCP socket placeholder (`wxSocket`).
pub struct Socket {
    host: String,
    port: u16,
    connected: bool,
    on_event: SocketEventHandler,
}

impl Default for Socket {
    fn default() -> Self {
        Self {
            host: String::new(),
            port: 0,
            connected: false,
            on_event: std::cell::RefCell::new(None),
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
        self.host = host.to_string();
        self.port = port;
        self.connected = true;
        Ok(())
    }

    pub fn is_connected(&self) -> bool {
        self.connected
    }

    pub fn endpoint(&self) -> (&str, u16) {
        (&self.host, self.port)
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

/// HTTP request placeholder (`wxWebRequest`).
#[derive(Debug, Clone)]
pub struct WebRequest {
    url: String,
    method: String,
}

impl WebRequest {
    pub fn get(url: &str) -> Self {
        Self {
            url: url.to_string(),
            method: "GET".into(),
        }
    }

    pub fn post(url: &str) -> Self {
        Self {
            url: url.to_string(),
            method: "POST".into(),
        }
    }

    pub fn url(&self) -> &str {
        &self.url
    }

    pub fn execute(&self) -> io::Result<Vec<u8>> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            format!("WebRequest stub: {} {}", self.method, self.url),
        ))
    }
}
