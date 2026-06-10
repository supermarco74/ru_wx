//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Zlib-compressed input stream (`wxZlibInputStream`).

use std::io;

use crate::io::memory_input_stream::MemoryInputStream;
use crate::io::stream::WxInputStream;

/// Decompressed read wrapper (`wxZlibInputStream`).
///
/// Stores decompressed payload; callers pass already-expanded bytes until
/// a native zlib backend is wired.
pub struct ZlibInputStream {
    inner: MemoryInputStream,
}

impl ZlibInputStream {
    pub fn from_decompressed(data: Vec<u8>) -> Self {
        Self {
            inner: MemoryInputStream::new(data),
        }
    }

    pub fn from_compressed_stub(compressed: Vec<u8>) -> Self {
        Self::from_decompressed(compressed)
    }
}

impl WxInputStream for ZlibInputStream {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.inner.read(buf)
    }

    fn eof(&self) -> bool {
        self.inner.eof()
    }
}
