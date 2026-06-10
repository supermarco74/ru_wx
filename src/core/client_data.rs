//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Per-item client data (`wxClientData`).

use crate::core::variant::Variant;

/// User data attached to controls (`wxClientData`).
pub trait ClientData: std::fmt::Debug {
    fn clone_box(&self) -> Box<dyn ClientData>;
}

/// String payload (`wxStringClientData`).
#[derive(Debug, Clone, Default)]
pub struct StringClientData {
    pub text: String,
}

impl StringClientData {
    pub fn new(text: &str) -> Self {
        Self {
            text: text.to_string(),
        }
    }
}

impl ClientData for StringClientData {
    fn clone_box(&self) -> Box<dyn ClientData> {
        Box::new(self.clone())
    }
}

/// Object payload stored as a [`Variant`] (`wxObjectClientData`).
#[derive(Debug, Clone)]
pub struct ObjectClientData {
    pub value: Variant,
}

impl ObjectClientData {
    pub fn new(value: Variant) -> Self {
        Self { value }
    }
}

impl ClientData for ObjectClientData {
    fn clone_box(&self) -> Box<dyn ClientData> {
        Box::new(self.clone())
    }
}
