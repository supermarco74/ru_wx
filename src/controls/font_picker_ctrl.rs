//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Font picker button (`wxFontPickerCtrl`).

use crate::controls::button::Button;
use crate::core::font::FontDesc;
use crate::core::widget::{WidgetRef, Window};
use crate::dialogs::font_dialog::FontDialog;
use crate::window::frame::Frame;

#[derive(Clone)]
pub struct FontPickerCtrl {
    button: Button,
    desc: std::rc::Rc<std::cell::RefCell<FontDesc>>,
}

impl FontPickerCtrl {
    pub fn new<W: Window>(parent: &W, frame: &Frame) -> Self {
        let desc = std::rc::Rc::new(std::cell::RefCell::new(FontDesc::default()));
        let button = Button::new(parent, "Pick font…");
        let d = desc.clone();
        let b = button.clone();
        let f = frame.clone();
        let bid = button.id();
        frame.register_command_handler(
            bid,
            Box::new(move || {
                let mut dlg = FontDialog::new(&f);
                dlg.set_initial_font(d.borrow().clone());
                if let Some(font) = dlg.show_modal() {
                    *d.borrow_mut() = font.desc().clone();
                    b.set_label(&d.borrow().face_name);
                }
            }),
        );
        Self { button, desc }
    }

    pub fn selected_font(&self) -> FontDesc {
        self.desc.borrow().clone()
    }

    pub fn as_widget_ref(&self) -> WidgetRef {
        self.button.as_widget_ref()
    }
}
