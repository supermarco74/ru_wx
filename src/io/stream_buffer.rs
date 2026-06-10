//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! In-memory stream buffer (`wxStreamBuffer`).

use std::io;

use crate::io::stream::{WxInputStream, WxOutputStream};

/// Growable byte buffer with read/write cursors (`wxStreamBuffer`).
#[derive(Debug, Default)]
pub struct StreamBuffer {
    data: Vec<u8>,
    read_pos: usize,
    write_pos: usize,
}

impl StreamBuffer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_capacity(cap: usize) -> Self {
        Self {
            data: Vec::with_capacity(cap),
            read_pos: 0,
            write_pos: 0,
        }
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.data
    }

    pub fn reset_read(&mut self) {
        self.read_pos = 0;
    }
}

impl WxInputStream for StreamBuffer {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if self.read_pos >= self.data.len() {
            return Ok(0);
        }
        let n = std::cmp::min(buf.len(), self.data.len() - self.read_pos);
        buf[..n].copy_from_slice(&self.data[self.read_pos..self.read_pos + n]);
        self.read_pos += n;
        Ok(n)
    }

    fn eof(&self) -> bool {
        self.read_pos >= self.data.len()
    }
}

impl WxOutputStream for StreamBuffer {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if self.write_pos == self.data.len() {
            self.data.extend_from_slice(buf);
        } else {
            let end = self.write_pos + buf.len();
            if end > self.data.len() {
                self.data.resize(end, 0);
            }
            self.data[self.write_pos..end].copy_from_slice(buf);
        }
        self.write_pos += buf.len();
        Ok(buf.len())
    }
}
