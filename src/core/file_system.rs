//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Virtual file system (`wxFileSystem`).

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Lightweight file-system helper (`wxFileSystem`).
#[derive(Debug, Default)]
pub struct FileSystem;

impl FileSystem {
    pub fn new() -> Self {
        Self
    }

    pub fn read_file(&self, path: impl AsRef<Path>) -> io::Result<String> {
        fs::read_to_string(path)
    }

    pub fn write_file(&self, path: impl AsRef<Path>, contents: &str) -> io::Result<()> {
        fs::write(path, contents)
    }

    pub fn exists(&self, path: impl AsRef<Path>) -> bool {
        path.as_ref().exists()
    }

    pub fn normalize(&self, path: impl AsRef<Path>) -> PathBuf {
        path.as_ref().to_path_buf()
    }

    /// Read a `memory:` URL via [`super::memory_fs_handler::MemoryFSHandler`].
    pub fn read_memory(&self, name: &str) -> Option<Vec<u8>> {
        super::memory_fs_handler::MemoryFSHandler::new().get_file(name)
    }

    /// Read an `archive:` URL via [`super::archive_fs_handler::ArchiveFSHandler`].
    pub fn read_archive(&self, name: &str) -> Option<Vec<u8>> {
        super::archive_fs_handler::ArchiveFSHandler::new().get_file(name)
    }

    /// Read a `zip:` URL via [`super::zip_fs_handler::ZipFSHandler`].
    pub fn read_zip(&self, path: &str) -> Option<Vec<u8>> {
        super::zip_fs_handler::ZipFSHandler::new().get_entry(path)
    }

    /// Read an `http:` / `https:` stub URL via [`super::internet_fs_handler::InternetFSHandler`].
    pub fn read_internet(&self, url: &str) -> Option<Vec<u8>> {
        super::internet_fs_handler::InternetFSHandler::new().fetch_stub(url)
    }
}

/// In-memory stream entry for virtual FS paths.
#[derive(Debug, Clone)]
pub struct FileSystemStream {
    pub url: String,
    pub data: Vec<u8>,
}

impl FileSystemStream {
    pub fn new(url: &str, data: Vec<u8>) -> Self {
        Self {
            url: url.to_string(),
            data,
        }
    }

    pub fn as_text(&self) -> io::Result<String> {
        String::from_utf8(self.data.clone()).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }
}
