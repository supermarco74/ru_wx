//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Read-only item container (`wxItemContainerImmutable`).

/// Read-only string list (`wxItemContainerImmutable`).
pub trait ItemContainerImmutable {
    fn count(&self) -> usize;
    fn get_string(&self, index: usize) -> Option<String>;
}

