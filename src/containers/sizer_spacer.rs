//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Fixed sizer spacer (`wxSizerSpacer`).

/// Empty space item for sizers (`wxSizerSpacer`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SizerSpacer {
    pub size: i32,
}

impl SizerSpacer {
    pub fn new(size: i32) -> Self {
        Self {
            size: if size < 0 { 0 } else { size },
        }
    }

    pub fn pixels(self) -> i32 {
        self.size
    }
}
