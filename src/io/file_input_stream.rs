//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! File input stream (`wxFileInputStream`).

use std::fs::File;
use std::io;
use std::path::Path;

use crate::io::stream::WxInputStream;

/// Read-only file stream (`wxFileInputStream`).
pub struct FileInputStream {
    file: File,
}

impl FileInputStream {
    pub fn open(path: impl AsRef<Path>) -> io::Result<Self> {
        Ok(Self {
            file: File::open(path)?,
        })
    }
}

impl WxInputStream for FileInputStream {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        use std::io::Read;
        self.file.read(buf)
    }

    fn eof(&self) -> bool {
        false
    }
}
