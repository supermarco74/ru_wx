//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Runtime type metadata (`wxClassInfo`).

/// Describes a wx-like class name (`wxClassInfo`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClassInfo {
    pub name: &'static str,
    pub base: Option<&'static str>,
}

impl ClassInfo {
    pub const fn new(name: &'static str) -> Self {
        Self { name, base: None }
    }

    pub const fn with_base(name: &'static str, base: &'static str) -> Self {
        Self {
            name,
            base: Some(base),
        }
    }

    pub fn is_kind_of(&self, ancestor: &str) -> bool {
        if self.name == ancestor {
            return true;
        }
        self.base == Some(ancestor)
    }
}
