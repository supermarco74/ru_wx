//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Directory path picker (`wxDirPickerCtrl`).

use crate::controls::button::Button;
use crate::controls::text_ctrl::TextCtrl;
use crate::core::widget::{WidgetRef, Window};
use crate::dialogs::dir_dialog::DirDialog;
use crate::window::frame::Frame;

#[derive(Clone)]
pub struct DirPickerCtrl {
    path: TextCtrl,
    browse: Button,
}

impl DirPickerCtrl {
    pub fn new<W: Window>(parent: &W, frame: &Frame) -> Self {
        let path = TextCtrl::new(parent, "");
        let browse = Button::new(parent, "…");
        let p = path.clone();
        let f = frame.clone();
        let bid = browse.id();
        frame.register_command_handler(
            bid,
            Box::new(move || {
                let mut dlg = DirDialog::new(&f);
                if let Some(chosen) = dlg.show_modal() {
                    p.set_value(&chosen);
                }
            }),
        );
        Self { path, browse }
    }

    pub fn path(&self) -> String {
        self.path.get_value()
    }

    pub fn set_path(&self, path: &str) {
        self.path.set_value(path);
    }

    pub fn path_widget(&self) -> WidgetRef {
        self.path.as_widget_ref()
    }

    pub fn browse_widget(&self) -> WidgetRef {
        self.browse.as_widget_ref()
    }
}
