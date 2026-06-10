//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Combo with custom popup (`wxPopupCtrl`).

use crate::controls::combo_ctrl::ComboCtrl;
use crate::core::widget::{WidgetRef, Window};

/// Text field with detachable popup (`wxPopupCtrl`).
#[derive(Clone)]
pub struct PopupCtrl {
    combo: ComboCtrl,
}

impl PopupCtrl {
    pub fn new<W: Window>(parent: &W) -> Self {
        Self {
            combo: ComboCtrl::new(parent),
        }
    }

    pub fn set_value(&self, value: &str) {
        self.combo.set_value(value);
    }

    pub fn value(&self) -> String {
        self.combo.value()
    }

    pub fn as_widget_ref(&self) -> WidgetRef {
        self.combo.as_widget_ref()
    }
}
