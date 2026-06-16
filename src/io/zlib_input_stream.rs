//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Zlib-compressed input stream (`wxZlibInputStream`).

use std::io;

use flate2::read::ZlibDecoder;
use std::io::Read;

use crate::io::memory_input_stream::MemoryInputStream;
use crate::io::stream::WxInputStream;

/// Decompressed read wrapper (`wxZlibInputStream`).
pub struct ZlibInputStream {
    inner: MemoryInputStream,
}

impl ZlibInputStream {
    pub fn from_decompressed(data: Vec<u8>) -> Self {
        Self {
            inner: MemoryInputStream::new(data),
        }
    }

    pub fn from_compressed(compressed: Vec<u8>) -> io::Result<Self> {
        let mut decoder = ZlibDecoder::new(compressed.as_slice());
        let mut out = Vec::new();
        decoder.read_to_end(&mut out)?;
        Ok(Self::from_decompressed(out))
    }

    /// Back-compat alias for older call sites.
    pub fn from_compressed_stub(compressed: Vec<u8>) -> Self {
        match Self::from_compressed(compressed.clone()) {
            Ok(s) => s,
            Err(_) => Self::from_decompressed(compressed),
        }
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
