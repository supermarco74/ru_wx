//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Text line streams (`wxTextInputStream`, `wxTextOutputStream`).

use std::io;

use crate::io::memory_input_stream::MemoryInputStream;
use crate::io::memory_stream::MemoryOutputStream;
use crate::io::stream::{WxInputStream, WxOutputStream};

/// Line-oriented reader over a byte stream (`wxTextInputStream`).
pub struct TextInputStream {
    inner: MemoryInputStream,
    line_buf: Vec<u8>,
}

impl TextInputStream {
    pub fn new(data: Vec<u8>) -> Self {
        Self {
            inner: MemoryInputStream::new(data),
            line_buf: Vec::new(),
        }
    }

    /// Build a stream over the UTF-8 bytes of `s`.
    // Infallible constructor mirroring wxTextInputStream usage; not
    // the `std::str::FromStr` trait (which would require an `Err`).
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Self {
        Self::new(s.as_bytes().to_vec())
    }

    /// Read the next line (without trailing `\n` / `\r\n`).
    pub fn read_line(&mut self) -> io::Result<Option<String>> {
        self.line_buf.clear();
        let mut byte = [0u8; 1];
        loop {
            let n = self.inner.read(&mut byte)?;
            if n == 0 {
                return if self.line_buf.is_empty() {
                    Ok(None)
                } else {
                    Ok(Some(String::from_utf8_lossy(&self.line_buf).into_owned()))
                };
            }
            if byte[0] == b'\n' {
                if self.line_buf.last() == Some(&b'\r') {
                    self.line_buf.pop();
                }
                return Ok(Some(String::from_utf8_lossy(&self.line_buf).into_owned()));
            }
            // Accumulate raw bytes: a UTF-8 code point may span
            // multiple reads, so decoding happens once per line.
            self.line_buf.push(byte[0]);
        }
    }
}

impl From<&str> for TextInputStream {
    fn from(s: &str) -> Self {
        Self::from_str(s)
    }
}

/// Line-oriented writer (`wxTextOutputStream`).
pub struct TextOutputStream {
    inner: MemoryOutputStream,
}

impl Default for TextOutputStream {
    fn default() -> Self {
        Self::new()
    }
}

impl TextOutputStream {
    pub fn new() -> Self {
        Self {
            inner: MemoryOutputStream::new(),
        }
    }

    pub fn write_line(&mut self, line: &str) -> io::Result<()> {
        let mut payload = line.to_string();
        payload.push('\n');
        self.inner.write(payload.as_bytes())?;
        Ok(())
    }

    pub fn into_bytes(self) -> Vec<u8> {
        self.inner.into_vec()
    }
}
