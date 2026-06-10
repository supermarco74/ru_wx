//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Toggle bitmap button (`wxBitmapToggleButton`).

use std::cell::RefCell;
use std::rc::Rc;

use crate::controls::bitmap_button::BitmapButton;
use crate::dc::bitmap::Bitmap;
use crate::core::widget::{WidgetRef, Window};
use crate::window::frame::Frame;

/// Checkable bitmap button (`wxBitmapToggleButton`).
#[derive(Clone)]
pub struct BitmapToggleButton {
    button: BitmapButton,
    checked: Rc<RefCell<bool>>,
}

impl BitmapToggleButton {
    pub fn new<W: Window>(parent: &W, bitmap: &Bitmap, width: u32, height: u32) -> Self {
        Self {
            button: BitmapButton::new(parent, bitmap, width as i32, height as i32),
            checked: Rc::new(RefCell::new(false)),
        }
    }

    pub fn set_value(&self, checked: bool) {
        *self.checked.borrow_mut() = checked;
    }

    pub fn value(&self) -> bool {
        *self.checked.borrow()
    }

    pub fn on_click<F: FnMut(bool) + 'static>(&self, frame: &Frame, mut callback: F) {
        let checked = self.checked.clone();
        let this = self.clone();
        self.button.on_click(frame, move || {
            let new = !*checked.borrow();
            *checked.borrow_mut() = new;
            this.set_value(new);
            callback(new);
        });
    }

    pub fn as_widget_ref(&self) -> WidgetRef {
        self.button.as_widget_ref()
    }
}
