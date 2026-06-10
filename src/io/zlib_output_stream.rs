//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Zlib-compressed output stream (`wxZlibOutputStream`).

use std::io;

use crate::io::memory_stream::MemoryOutputStream;
use crate::io::stream::WxOutputStream;

/// Compressed write wrapper (`wxZlibOutputStream`).
///
/// Stores uncompressed payload; callers read raw bytes until a native
/// zlib backend is wired.
pub struct ZlibOutputStream {
    inner: MemoryOutputStream,
}

impl ZlibOutputStream {
    pub fn new() -> Self {
        Self {
            inner: MemoryOutputStream::new(),
        }
    }

    pub fn data(&self) -> &[u8] {
        self.inner.data()
    }

    pub fn into_vec(self) -> Vec<u8> {
        self.inner.into_vec()
    }
}

impl Default for ZlibOutputStream {
    fn default() -> Self {
        Self::new()
    }
}

impl WxOutputStream for ZlibOutputStream {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.inner.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}
