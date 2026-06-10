//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Sizer item flags (`wxSizerFlags`).

/// Fluent builder for sizer item placement (`wxSizerFlags`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[derive(Default)]
pub struct SizerFlags {
    pub proportion: u32,
    pub border: i32,
    pub expand: bool,
    pub center: bool,
}


impl SizerFlags {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn proportion(mut self, proportion: u32) -> Self {
        self.proportion = proportion;
        self
    }

    pub fn border(mut self, pixels: i32) -> Self {
        self.border = pixels.max(0);
        self
    }

    pub fn expand(mut self) -> Self {
        self.expand = true;
        self
    }

    pub fn center(mut self) -> Self {
        self.center = true;
        self
    }
}
