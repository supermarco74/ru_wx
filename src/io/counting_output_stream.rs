//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Counting output stream (`wxCountingOutputStream`).

use std::io;

use crate::io::memory_stream::MemoryOutputStream;
use crate::io::stream::WxOutputStream;

/// Counts bytes written without storing payload (`wxCountingOutputStream`).
pub struct CountingOutputStream {
    inner: MemoryOutputStream,
    count: u64,
}

impl CountingOutputStream {
    pub fn new() -> Self {
        Self {
            inner: MemoryOutputStream::new(),
            count: 0,
        }
    }

    pub fn bytes_written(&self) -> u64 {
        self.count
    }
}

impl Default for CountingOutputStream {
    fn default() -> Self {
        Self::new()
    }
}

impl WxOutputStream for CountingOutputStream {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let n = self.inner.write(buf)?;
        self.count += n as u64;
        Ok(n)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}
