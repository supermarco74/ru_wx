//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Network protocol descriptor (`wxProtocol`).

/// Supported URL / stream protocol (`wxProtocol`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Protocol {
    File,
    Http,
    Https,
    Ftp,
    Memory,
    Archive,
    Zip,
    Unknown,
}

impl Protocol {
    pub fn from_scheme(scheme: &str) -> Self {
        match scheme.to_ascii_lowercase().as_str() {
            "file" => Self::File,
            "http" => Self::Http,
            "https" => Self::Https,
            "ftp" => Self::Ftp,
            "memory" => Self::Memory,
            "archive" => Self::Archive,
            "zip" => Self::Zip,
            _ => Self::Unknown,
        }
    }

    pub fn scheme(&self) -> &'static str {
        match self {
            Self::File => "file",
            Self::Http => "http",
            Self::Https => "https",
            Self::Ftp => "ftp",
            Self::Memory => "memory",
            Self::Archive => "archive",
            Self::Zip => "zip",
            Self::Unknown => "",
        }
    }

    pub fn is_network(&self) -> bool {
        matches!(self, Self::Http | Self::Https | Self::Ftp)
    }
}
