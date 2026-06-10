//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! FFile-backed output stream (`wxFFileOutputStream`).

use std::io;
use std::path::Path;

use crate::io::stream::WxOutputStream;
use crate::io::wx_ffile::WxFFile;

/// Write-only stream backed by [`WxFFile`] (`wxFFileOutputStream`).
pub struct FFileOutputStream {
    file: WxFFile,
}

impl FFileOutputStream {
    pub fn create(path: impl AsRef<Path>) -> io::Result<Self> {
        Ok(Self {
            file: WxFFile::create(path)?,
        })
    }

    pub fn append(path: impl AsRef<Path>) -> io::Result<Self> {
        Ok(Self {
            file: WxFFile::append(path)?,
        })
    }

    pub fn path(&self) -> &str {
        self.file.path()
    }
}

impl WxOutputStream for FFileOutputStream {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.file.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.file.flush()
    }
}
