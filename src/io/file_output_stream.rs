//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! File output stream (`wxFileOutputStream`).

use std::fs::File;
use std::io;
use std::path::Path;

use crate::io::stream::WxOutputStream;

/// Write-only file stream (`wxFileOutputStream`).
pub struct FileOutputStream {
    file: File,
}

impl FileOutputStream {
    pub fn create(path: impl AsRef<Path>) -> io::Result<Self> {
        Ok(Self {
            file: File::create(path)?,
        })
    }

    pub fn append(path: impl AsRef<Path>) -> io::Result<Self> {
        use std::fs::OpenOptions;
        Ok(Self {
            file: OpenOptions::new().append(true).create(true).open(path)?,
        })
    }
}

impl WxOutputStream for FileOutputStream {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        use std::io::Write;
        self.file.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        use std::io::Write;
        self.file.flush()
    }
}
