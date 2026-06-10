//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Type-erased value (`wxAny`).

use crate::core::variant::Variant;

/// Holds any serialisable value (`wxAny`).
#[derive(Debug, Clone, Default)]
pub struct WxAny {
    value: Option<Variant>,
}

impl WxAny {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_variant(value: Variant) -> Self {
        Self {
            value: Some(value),
        }
    }

    pub fn clear(&mut self) {
        self.value = None;
    }

    pub fn is_empty(&self) -> bool {
        self.value.is_none()
    }

    pub fn as_variant(&self) -> Option<&Variant> {
        self.value.as_ref()
    }

    pub fn into_variant(self) -> Option<Variant> {
        self.value
    }
}

impl From<bool> for WxAny {
    fn from(value: bool) -> Self {
        Self::from_variant(Variant::from(value))
    }
}

impl From<i64> for WxAny {
    fn from(value: i64) -> Self {
        Self::from_variant(Variant::from(value))
    }
}

impl From<&str> for WxAny {
    fn from(value: &str) -> Self {
        Self::from_variant(Variant::from(value))
    }
}
