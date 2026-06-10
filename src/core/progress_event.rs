//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Progress dialog / gauge event (`wxProgressEvent`).

/// Progress range update (`wxProgressEvent`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProgressEvent {
    pub value: i32,
    pub maximum: i32,
    pub skipped: bool,
}

impl ProgressEvent {
    pub const fn new(value: i32, maximum: i32) -> Self {
        Self {
            value,
            maximum,
            skipped: false,
        }
    }

    pub const fn skipped() -> Self {
        Self {
            value: 0,
            maximum: 0,
            skipped: true,
        }
    }

    pub fn fraction(&self) -> f64 {
        if self.maximum <= 0 {
            0.0
        } else {
            (self.value as f64 / self.maximum as f64).clamp(0.0, 1.0)
        }
    }
}

