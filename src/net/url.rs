//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Parsed URL (`wxURL`).

use crate::net::protocol::Protocol;

/// Decomposed URL (`wxURL`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Url {
    pub protocol: Protocol,
    pub host: String,
    pub path: String,
    pub raw: String,
}

impl Url {
    pub fn parse(raw: &str) -> Option<Self> {
        let (scheme, rest) = raw.split_once("://")?;
        let protocol = Protocol::from_scheme(scheme);
        let (host, path) = match rest.split_once('/') {
            Some((h, p)) => (h.to_string(), format!("/{p}")),
            None => (rest.to_string(), "/".to_string()),
        };
        Some(Self {
            protocol,
            host,
            path,
            raw: raw.to_string(),
        })
    }

    pub fn protocol(&self) -> Protocol {
        self.protocol
    }

    pub fn host(&self) -> &str {
        &self.host
    }

    pub fn path(&self) -> &str {
        &self.path
    }

    pub fn rebuild(&self) -> String {
        if self.raw.is_empty() {
            format!("{}://{}{}", self.protocol.scheme(), self.host, self.path)
        } else {
            self.raw.clone()
        }
    }
}

impl std::fmt::Display for Url {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.rebuild())
    }
}
