//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Stream error descriptor (`wxStreamError`).

use std::io;

/// Stream failure code (`wxStreamError`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StreamError {
    pub message: String,
    pub kind: io::ErrorKind,
}

impl StreamError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            kind: io::ErrorKind::Other,
        }
    }

    pub fn from_io(err: io::Error) -> Self {
        Self {
            message: err.to_string(),
            kind: err.kind(),
        }
    }

    pub fn into_io_error(self) -> io::Error {
        io::Error::new(self.kind, self.message)
    }
}
