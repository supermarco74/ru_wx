//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Counting input stream (`wxCountingInputStream`).

use std::io;

use crate::io::memory_input_stream::MemoryInputStream;
use crate::io::stream::WxInputStream;

/// Counts bytes read from another stream (`wxCountingInputStream`).
pub struct CountingInputStream {
    inner: MemoryInputStream,
    count: u64,
}

impl CountingInputStream {
    pub fn new(data: Vec<u8>) -> Self {
        Self {
            inner: MemoryInputStream::new(data),
            count: 0,
        }
    }

    pub fn bytes_read(&self) -> u64 {
        self.count
    }
}

impl WxInputStream for CountingInputStream {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let n = self.inner.read(buf)?;
        self.count += n as u64;
        Ok(n)
    }

    fn eof(&self) -> bool {
        self.inner.eof()
    }
}
