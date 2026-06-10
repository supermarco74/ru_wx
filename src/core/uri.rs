//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! URI parsing (`wxURI`).

use std::fmt;

/// Parsed URI (`wxURI`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Uri {
    pub scheme: String,
    pub host: String,
    pub path: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UriError;

impl fmt::Display for UriError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("invalid URI")
    }
}

impl std::error::Error for UriError {}

impl Uri {
    pub fn parse(s: &str) -> Result<Self, UriError> {
        let (scheme, rest) = s.split_once("://").ok_or(UriError)?;
        if scheme.is_empty() {
            return Err(UriError);
        }
        let (host, path) = match rest.split_once('/') {
            Some((h, p)) => (h.to_string(), format!("/{p}")),
            None => (rest.to_string(), String::new()),
        };
        Ok(Self {
            scheme: scheme.to_string(),
            host,
            path,
        })
    }

    pub fn to_string_lossy(&self) -> String {
        if self.path.is_empty() {
            format!("{}://{}", self.scheme, self.host)
        } else {
            format!("{}://{}{}", self.scheme, self.host, self.path)
        }
    }
}
