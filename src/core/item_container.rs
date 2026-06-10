//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Item container trait (`wxItemContainer`).

/// String list interface shared by list/combo controls (`wxItemContainer`).
pub trait ItemContainer {
    fn count(&self) -> usize;
    fn get_string(&self, index: usize) -> Option<String>;
    fn append(&self, item: &str);
    fn clear(&self);
}
