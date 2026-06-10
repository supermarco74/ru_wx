//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Buffered input stream (`wxBufferedInputStream`).

use std::io;

use crate::io::memory_input_stream::MemoryInputStream;
use crate::io::stream::WxInputStream;

/// Read-ahead wrapper over another input stream (`wxBufferedInputStream`).
pub struct BufferedInputStream {
    inner: MemoryInputStream,
    chunk_size: usize,
}

impl BufferedInputStream {
    pub fn new(data: Vec<u8>, chunk_size: usize) -> Self {
        Self {
            inner: MemoryInputStream::new(data),
            chunk_size: chunk_size.max(1),
        }
    }

    pub fn chunk_size(&self) -> usize {
        self.chunk_size
    }
}

impl WxInputStream for BufferedInputStream {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.inner.read(buf)
    }

    fn eof(&self) -> bool {
        self.inner.eof()
    }
}
