//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Short-lived popup (`wxPopupTransientWindow`).

use crate::core::geometry::Rect;
use crate::window::frame::Frame;
use crate::window::frame_extras::TipWindow;

/// Auto-dismissing transient popup (`wxPopupTransientWindow`).
#[derive(Clone)]
pub struct PopupTransientWindow {
    tip: TipWindow,
}

impl PopupTransientWindow {
    pub fn new(parent: &Frame, text: &str, anchor: Rect) -> Self {
        let tip = TipWindow::new(parent, anchor, text);
        Self { tip }
    }

    pub fn set_text(&self, text: &str) {
        self.tip.set_text(text);
    }

    pub fn close(&self) {
        self.tip.close();
    }

    pub fn tip(&self) -> &TipWindow {
        &self.tip
    }
}
