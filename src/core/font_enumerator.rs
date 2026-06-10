//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Font enumeration (`wxFontEnumerator`).

use crate::core::font::FontDesc;

/// Collect installed font faces (`wxFontEnumerator`).
#[derive(Debug, Default)]
pub struct FontEnumerator {
    faces: Vec<String>,
}

impl FontEnumerator {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn enumerate(&mut self) -> &[String] {
        if self.faces.is_empty() {
            self.faces = vec![
                "Segoe UI".into(),
                "Arial".into(),
                "Courier New".into(),
                "Times New Roman".into(),
            ];
        }
        &self.faces
    }

    pub fn default_desc(&self) -> FontDesc {
        FontDesc::new("Segoe UI", 10)
    }
}
