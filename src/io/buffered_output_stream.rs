//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Buffered output stream (`wxBufferedOutputStream`).

use std::io;

use crate::io::memory_stream::MemoryOutputStream;
use crate::io::stream::WxOutputStream;

/// Write-behind wrapper over another output stream (`wxBufferedOutputStream`).
pub struct BufferedOutputStream {
    inner: MemoryOutputStream,
    chunk_size: usize,
}

impl BufferedOutputStream {
    pub fn new(chunk_size: usize) -> Self {
        Self {
            inner: MemoryOutputStream::new(),
            chunk_size: chunk_size.max(1),
        }
    }

    pub fn chunk_size(&self) -> usize {
        self.chunk_size
    }

    pub fn data(&self) -> &[u8] {
        self.inner.data()
    }

    pub fn into_vec(self) -> Vec<u8> {
        self.inner.into_vec()
    }
}

impl WxOutputStream for BufferedOutputStream {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.inner.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}
