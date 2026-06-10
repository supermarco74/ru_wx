//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Stream base helpers (`wxStreamBase`).

use std::io;

use crate::io::stream::{WxInputStream, WxOutputStream};

/// Common stream status (`wxStreamBase`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StreamBase {
    last_error: Option<String>,
}

impl StreamBase {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn last_error(&self) -> Option<&str> {
        self.last_error.as_deref()
    }

    pub fn is_ok(&self) -> bool {
        self.last_error.is_none()
    }

    pub fn set_error(&mut self, message: impl Into<String>) {
        self.last_error = Some(message.into());
    }

    pub fn clear_error(&mut self) {
        self.last_error = None;
    }

    pub fn record_io_error(&mut self, err: io::Error) {
        self.last_error = Some(err.to_string());
    }
}

/// Extension helpers for input streams.
pub trait InputStreamExt: WxInputStream {
    fn read_all(&mut self) -> io::Result<Vec<u8>> {
        let mut buf = Vec::new();
        let mut chunk = [0u8; 4096];
        loop {
            let n = self.read(&mut chunk)?;
            if n == 0 {
                break;
            }
            buf.extend_from_slice(&chunk[..n]);
            if self.eof() {
                break;
            }
        }
        Ok(buf)
    }
}

impl<T: WxInputStream + ?Sized> InputStreamExt for T {}

/// Extension helpers for output streams.
pub trait OutputStreamExt: WxOutputStream {
    fn write_all(&mut self, data: &[u8]) -> io::Result<()> {
        let mut offset = 0;
        while offset < data.len() {
            let n = self.write(&data[offset..])?;
            if n == 0 {
                return Err(io::Error::new(
                    io::ErrorKind::WriteZero,
                    "stream wrote zero bytes",
                ));
            }
            offset += n;
        }
        Ok(())
    }
}

impl<T: WxOutputStream + ?Sized> OutputStreamExt for T {}
