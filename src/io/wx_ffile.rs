//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! File handle wrapper (`wxFFile`) and offset type (`wxFileOffset`).

use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::Path;

/// Byte offset in a file (`wxFileOffset`).
pub type FileOffset = i64;

/// Low-level file I/O (`wxFFile`).
pub struct WxFFile {
    file: File,
    path: String,
}

impl WxFFile {
    pub fn open(path: impl AsRef<Path>) -> io::Result<Self> {
        let path = path.as_ref();
        let file = File::open(path)?;
        Ok(Self {
            file,
            path: path.display().to_string(),
        })
    }

    pub fn create(path: impl AsRef<Path>) -> io::Result<Self> {
        let path = path.as_ref();
        let file = File::create(path)?;
        Ok(Self {
            file,
            path: path.display().to_string(),
        })
    }

    pub fn append(path: impl AsRef<Path>) -> io::Result<Self> {
        use std::fs::OpenOptions;
        let path = path.as_ref();
        let file = OpenOptions::new().append(true).create(true).open(path)?;
        Ok(Self {
            file,
            path: path.display().to_string(),
        })
    }

    pub fn path(&self) -> &str {
        &self.path
    }

    pub fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.file.read(buf)
    }

    pub fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.file.write(buf)
    }

    pub fn seek(&mut self, offset: FileOffset) -> io::Result<FileOffset> {
        self.file.seek(SeekFrom::Start(offset as u64)).map(|p| p as FileOffset)
    }

    pub fn length(&mut self) -> io::Result<FileOffset> {
        let pos = self.file.stream_position()? as FileOffset;
        let len = self.file.seek(SeekFrom::End(0))? as FileOffset;
        self.file.seek(SeekFrom::Start(pos as u64))?;
        Ok(len)
    }

    pub fn flush(&mut self) -> io::Result<()> {
        self.file.flush()
    }
}
