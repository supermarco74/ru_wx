//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Native file path entry (`wxFileCtrl`).

use crate::controls::text_ctrl::TextCtrl;
use crate::core::widget::{WidgetRef, Window};

#[derive(Clone)]
pub struct FileCtrl {
    path: TextCtrl,
}

impl FileCtrl {
    pub fn new<W: Window>(parent: &W) -> Self {
        Self {
            path: TextCtrl::new(parent, ""),
        }
    }

    pub fn filename(&self) -> String {
        self.path.get_value()
    }

    pub fn set_filename(&self, path: &str) {
        self.path.set_value(path);
    }

    pub fn as_widget_ref(&self) -> WidgetRef {
        self.path.as_widget_ref()
    }
}
