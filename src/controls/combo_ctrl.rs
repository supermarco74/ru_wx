//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Customizable combo (`wxComboCtrl`).

use crate::controls::combo_box::ComboBox;
use crate::core::widget::{WidgetRef, Window};

/// Combo with popup customization hook (`wxComboCtrl`).
#[derive(Clone)]
pub struct ComboCtrl {
    combo: ComboBox,
}

impl ComboCtrl {
    pub fn new<W: Window>(parent: &W) -> Self {
        Self {
            combo: ComboBox::new(parent),
        }
    }

    pub fn append(&self, item: &str) {
        self.combo.append(item);
    }

    pub fn selection(&self) -> Option<usize> {
        self.combo.get_selection()
    }

    pub fn set_selection(&self, index: usize) {
        self.combo.set_selection(index);
    }

    pub fn value(&self) -> String {
        self.combo.get_value()
    }

    pub fn set_value(&self, text: &str) {
        self.combo.set_value(text);
    }

    pub fn as_widget_ref(&self) -> WidgetRef {
        self.combo.as_widget_ref()
    }
}
