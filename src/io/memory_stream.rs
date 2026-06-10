//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! In-memory output stream (`wxMemoryOutputStream`).

use std::io;

use crate::io::stream::WxOutputStream;

/// Growable memory buffer (`wxMemoryOutputStream`).
#[derive(Debug, Default)]
pub struct MemoryOutputStream {
    buffer: Vec<u8>,
}

impl MemoryOutputStream {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn data(&self) -> &[u8] {
        &self.buffer
    }

    pub fn into_vec(self) -> Vec<u8> {
        self.buffer
    }
}

impl WxOutputStream for MemoryOutputStream {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.buffer.extend_from_slice(buf);
        Ok(buf.len())
    }
}
