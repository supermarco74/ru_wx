//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! IPC server (`wxServer`).

use std::collections::VecDeque;
use std::io;

/// Server endpoint for IPC (`wxServer`).
#[derive(Debug, Default)]
pub struct IpcServer {
    service: String,
    inbox: VecDeque<Vec<u8>>,
    listening: bool,
}

impl IpcServer {
    pub fn new(service: &str) -> Self {
        Self {
            service: service.to_string(),
            inbox: VecDeque::new(),
            listening: false,
        }
    }

    pub fn listen(&mut self) -> io::Result<()> {
        self.listening = true;
        Ok(())
    }

    pub fn is_listening(&self) -> bool {
        self.listening
    }

    pub fn push_message(&mut self, data: Vec<u8>) {
        self.inbox.push_back(data);
    }

    pub fn receive(&mut self) -> Option<Vec<u8>> {
        self.inbox.pop_front()
    }

    pub fn service(&self) -> &str {
        &self.service
    }
}
