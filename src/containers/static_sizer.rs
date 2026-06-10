//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Fixed-size sizer (`wxStaticSizer`).

use crate::containers::sizer::{BoxSizer, Orientation};

/// Sizer composed only of fixed spacers (`wxStaticSizer`).
pub struct StaticSizer {
    inner: BoxSizer,
}

impl StaticSizer {
    pub fn vertical() -> Self {
        Self {
            inner: BoxSizer::vertical(),
        }
    }

    pub fn horizontal() -> Self {
        Self {
            inner: BoxSizer::horizontal(),
        }
    }

    pub fn add_spacer(&mut self, size: i32) {
        self.inner.add_spacer(size);
    }

    pub fn layout(&mut self, x: i32, y: i32, width: u32, height: u32) {
        self.inner.layout(x, y, width, height);
    }

    pub fn orientation(&self) -> Orientation {
        self.inner.orientation()
    }
}
