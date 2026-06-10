//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Tee input stream (`wxTeeInputStream`).

use std::io;

use crate::io::memory_input_stream::MemoryInputStream;
use crate::io::memory_stream::MemoryOutputStream;
use crate::io::stream::WxInputStream;
use crate::io::stream::WxOutputStream;

/// Reads from one stream and copies to another (`wxTeeInputStream`).
pub struct TeeInputStream {
    inner: MemoryInputStream,
    tee: MemoryOutputStream,
}

impl TeeInputStream {
    pub fn new(data: Vec<u8>) -> Self {
        Self {
            inner: MemoryInputStream::new(data),
            tee: MemoryOutputStream::new(),
        }
    }

    pub fn tee_data(&self) -> &[u8] {
        self.tee.data()
    }
}

impl WxInputStream for TeeInputStream {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let n = self.inner.read(buf)?;
        if n > 0 {
            let _ = self.tee.write(&buf[..n]);
        }
        Ok(n)
    }

    fn eof(&self) -> bool {
        self.inner.eof()
    }
}
