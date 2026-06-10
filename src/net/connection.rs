//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! IPC connection (`wxConnection`).

use std::collections::VecDeque;
use std::io;

/// Inter-process message pipe (`wxConnection`).
#[derive(Debug, Default)]
pub struct IpcConnection {
    name: String,
    inbox: VecDeque<Vec<u8>>,
    connected: bool,
}

impl IpcConnection {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            inbox: VecDeque::new(),
            connected: false,
        }
    }

    pub fn connect(&mut self) -> io::Result<()> {
        self.connected = true;
        Ok(())
    }

    pub fn disconnect(&mut self) {
        self.connected = false;
        self.inbox.clear();
    }

    pub fn is_connected(&self) -> bool {
        self.connected
    }

    pub fn send(&mut self, data: &[u8]) -> io::Result<()> {
        if !self.connected {
            return Err(io::Error::new(io::ErrorKind::NotConnected, "IPC disconnected"));
        }
        self.inbox.push_back(data.to_vec());
        Ok(())
    }

    pub fn receive(&mut self) -> Option<Vec<u8>> {
        self.inbox.pop_front()
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}
