//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Data-view bitmap renderer (`wxDataViewBitmapRenderer`).

use super::data_view::DataViewRenderer;

/// Renders icon indices in data views (`wxDataViewBitmapRenderer`).
#[derive(Debug, Clone, Copy, Default)]
pub struct DataViewBitmapRenderer {
    pub image_index: i32,
}

impl DataViewBitmapRenderer {
    pub fn new(image_index: i32) -> Self {
        Self { image_index }
    }
}

impl DataViewRenderer for DataViewBitmapRenderer {
    fn render_text(&self, _value: &str) -> String {
        format!("[img:{}]", self.image_index)
    }
}
