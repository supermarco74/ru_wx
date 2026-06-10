//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! FTP client stub (`wxFTP`).

use std::io;

/// FTP session placeholder (`wxFTP`).
#[derive(Debug, Default)]
pub struct FtpClient {
    host: String,
    connected: bool,
}

impl FtpClient {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn connect(&mut self, host: &str, _user: &str, _password: &str) -> io::Result<()> {
        self.host = host.to_string();
        self.connected = true;
        Ok(())
    }

    pub fn is_connected(&self) -> bool {
        self.connected
    }

    pub fn host(&self) -> &str {
        &self.host
    }

    pub fn get_file(&self, remote: &str) -> io::Result<Vec<u8>> {
        if !self.connected {
            return Err(io::Error::new(io::ErrorKind::NotConnected, "FTP not connected"));
        }
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            format!("FTP get stub: {remote}"),
        ))
    }
}
