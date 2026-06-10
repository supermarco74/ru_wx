//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Stream traits (`wxInputStream` / `wxOutputStream`).

use std::io;

/// Readable byte stream (`wxInputStream`).
pub trait WxInputStream {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize>;
    fn eof(&self) -> bool;
}

/// Writable byte stream (`wxOutputStream`).
pub trait WxOutputStream {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize>;
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
