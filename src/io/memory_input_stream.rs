//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! In-memory input stream (`wxMemoryInputStream`).

use std::io;

use crate::io::stream::WxInputStream;

/// Read-only memory buffer (`wxMemoryInputStream`).
#[derive(Debug)]
pub struct MemoryInputStream {
    data: Vec<u8>,
    pos: usize,
}

impl MemoryInputStream {
    pub fn new(data: Vec<u8>) -> Self {
        Self { data, pos: 0 }
    }

    /// Build a stream over the UTF-8 bytes of `s`.
    // Infallible constructor mirroring wxMemoryInputStream usage; not
    // the `std::str::FromStr` trait (which would require an `Err`).
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Self {
        Self::new(s.as_bytes().to_vec())
    }
}

impl From<&str> for MemoryInputStream {
    fn from(s: &str) -> Self {
        Self::from_str(s)
    }
}

impl WxInputStream for MemoryInputStream {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if self.pos >= self.data.len() {
            return Ok(0);
        }
        let n = std::cmp::min(buf.len(), self.data.len() - self.pos);
        buf[..n].copy_from_slice(&self.data[self.pos..self.pos + n]);
        self.pos += n;
        Ok(n)
    }

    fn eof(&self) -> bool {
        self.pos >= self.data.len()
    }
}
