//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! FFile-backed input stream (`wxFFileInputStream`).

use std::io;
use std::path::Path;

use crate::io::stream::WxInputStream;
use crate::io::wx_ffile::WxFFile;

/// Read-only stream backed by [`WxFFile`] (`wxFFileInputStream`).
pub struct FFileInputStream {
    file: WxFFile,
    eof: bool,
}

impl FFileInputStream {
    pub fn open(path: impl AsRef<Path>) -> io::Result<Self> {
        Ok(Self {
            file: WxFFile::open(path)?,
            eof: false,
        })
    }

    pub fn path(&self) -> &str {
        self.file.path()
    }
}

impl WxInputStream for FFileInputStream {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let n = self.file.read(buf)?;
        if n == 0 {
            self.eof = true;
        }
        Ok(n)
    }

    fn eof(&self) -> bool {
        self.eof
    }
}
