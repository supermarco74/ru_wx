//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Filter output stream (`wxFilterOutputStream`).

use std::io;

use crate::io::memory_stream::MemoryOutputStream;
use crate::io::stream::WxOutputStream;

/// Stream decorator with optional byte transform on write (`wxFilterOutputStream`).
pub struct FilterOutputStream {
    inner: MemoryOutputStream,
    append_cr: bool,
}

impl FilterOutputStream {
    pub fn new() -> Self {
        Self {
            inner: MemoryOutputStream::new(),
            append_cr: false,
        }
    }

    pub fn with_append_cr(mut self, append: bool) -> Self {
        self.append_cr = append;
        self
    }

    pub fn data(&self) -> &[u8] {
        self.inner.data()
    }

    pub fn into_vec(self) -> Vec<u8> {
        self.inner.into_vec()
    }
}

impl Default for FilterOutputStream {
    fn default() -> Self {
        Self::new()
    }
}

impl WxOutputStream for FilterOutputStream {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if self.append_cr {
            let mut expanded = Vec::with_capacity(buf.len() * 2);
            for &b in buf {
                expanded.push(b);
                if b == b'\n' {
                    expanded.push(b'\r');
                }
            }
            return self.inner.write(&expanded);
        }
        self.inner.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}
