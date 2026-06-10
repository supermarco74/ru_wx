//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Version metadata (`wxVersionInfo`).

/// Application / library version descriptor (`wxVersionInfo`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VersionInfo {
    pub name: String,
    pub version: String,
    pub description: String,
    pub copyright: String,
}

impl VersionInfo {
    pub fn new(name: &str, version: &str) -> Self {
        Self {
            name: name.to_string(),
            version: version.to_string(),
            description: String::new(),
            copyright: String::new(),
        }
    }

    pub fn with_description(mut self, text: &str) -> Self {
        self.description = text.to_string();
        self
    }

    pub fn with_copyright(mut self, text: &str) -> Self {
        self.copyright = text.to_string();
        self
    }

    pub fn long_version_string(&self) -> String {
        if self.description.is_empty() {
            format!("{} {}", self.name, self.version)
        } else {
            format!("{} {} — {}", self.name, self.version, self.description)
        }
    }
}
