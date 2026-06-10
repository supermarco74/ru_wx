//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Credential storage (`wxSecretStore`).

use std::collections::HashMap;

/// Key/value secret vault (`wxSecretStore`) — in-memory stub.
#[derive(Debug, Default)]
pub struct SecretStore {
    secrets: HashMap<String, String>,
}

impl SecretStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn save(&mut self, service: &str, user: &str, secret: &str) -> bool {
        self.secrets
            .insert(format!("{service}:{user}"), secret.to_string());
        true
    }

    pub fn load(&self, service: &str, user: &str) -> Option<String> {
        self.secrets
            .get(&format!("{service}:{user}"))
            .cloned()
    }

    pub fn delete(&mut self, service: &str, user: &str) -> bool {
        self.secrets.remove(&format!("{service}:{user}")).is_some()
    }
}
