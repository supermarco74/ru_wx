//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! HTTP client stub (`wxHTTP`).

use std::io;

/// HTTP session placeholder (`wxHTTP`).
#[derive(Debug, Default)]
pub struct HttpClient {
    base_url: String,
}

impl HttpClient {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_base_url(&mut self, url: &str) {
        self.base_url = url.to_string();
    }

    pub fn get(&self, path: &str) -> io::Result<Vec<u8>> {
        let url = if path.starts_with("http") {
            path.to_string()
        } else {
            format!("{}{}", self.base_url.trim_end_matches('/'), path)
        };
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            format!("HTTP get stub: {url}"),
        ))
    }

    pub fn post(&self, path: &str, _body: &[u8]) -> io::Result<Vec<u8>> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            format!("HTTP post stub: {path}"),
        ))
    }
}
