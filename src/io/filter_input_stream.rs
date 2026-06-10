//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Filter input stream (`wxFilterInputStream`).

use std::io;

use crate::io::memory_input_stream::MemoryInputStream;
use crate::io::stream::WxInputStream;

/// Stream decorator with optional byte transform (`wxFilterInputStream`).
pub struct FilterInputStream {
    inner: MemoryInputStream,
    strip_cr: bool,
}

impl FilterInputStream {
    pub fn new(data: Vec<u8>) -> Self {
        Self {
            inner: MemoryInputStream::new(data),
            strip_cr: false,
        }
    }

    pub fn with_strip_cr(mut self, strip: bool) -> Self {
        self.strip_cr = strip;
        self
    }
}

impl WxInputStream for FilterInputStream {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let n = self.inner.read(buf)?;
        if self.strip_cr {
            let mut w = 0;
            for i in 0..n {
                if buf[i] != b'\r' {
                    buf[w] = buf[i];
                    w += 1;
                }
            }
            return Ok(w);
        }
        Ok(n)
    }

    fn eof(&self) -> bool {
        self.inner.eof()
    }
}
