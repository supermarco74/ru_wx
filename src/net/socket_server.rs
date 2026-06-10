//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! TCP server socket (`wxSocketServer`).

use std::io;
use std::net::{TcpListener, TcpStream};

/// Listening TCP socket (`wxSocketServer`).
pub struct SocketServer {
    listener: Option<TcpListener>,
    port: u16,
}

impl SocketServer {
    pub fn new() -> Self {
        Self {
            listener: None,
            port: 0,
        }
    }

    pub fn listen(&mut self, port: u16) -> io::Result<()> {
        let listener = TcpListener::bind(("127.0.0.1", port))?;
        self.port = port;
        self.listener = Some(listener);
        Ok(())
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub fn accept(&self) -> io::Result<TcpStream> {
        match &self.listener {
            Some(l) => {
                let (stream, _) = l.accept()?;
                Ok(stream)
            }
            None => Err(io::Error::new(
                io::ErrorKind::NotConnected,
                "SocketServer not listening",
            )),
        }
    }
}

impl Default for SocketServer {
    fn default() -> Self {
        Self::new()
    }
}
