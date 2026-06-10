//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! IPC client (`wxClient`).

use std::io;

use crate::net::connection::IpcConnection;

/// Client endpoint for IPC (`wxClient`).
#[derive(Debug)]
pub struct IpcClient {
    service: String,
    connection: IpcConnection,
}

impl IpcClient {
    pub fn new(service: &str) -> Self {
        Self {
            service: service.to_string(),
            connection: IpcConnection::new(service),
        }
    }

    pub fn connect(&mut self) -> io::Result<()> {
        self.connection.connect()
    }

    pub fn send(&mut self, data: &[u8]) -> io::Result<()> {
        self.connection.send(data)
    }

    pub fn receive(&mut self) -> Option<Vec<u8>> {
        self.connection.receive()
    }

    pub fn service(&self) -> &str {
        &self.service
    }
}
