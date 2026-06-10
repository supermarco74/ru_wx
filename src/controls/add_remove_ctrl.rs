//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! List with add/remove buttons (`wxAddRemoveCtrl`).

use crate::controls::button::Button;
use crate::controls::list_box::ListBox;
use crate::controls::text_ctrl::TextCtrl;
use crate::core::widget::{WidgetRef, Window};
use crate::window::frame::Frame;

#[derive(Clone)]
pub struct AddRemoveCtrl {
    list: ListBox,
    entry: TextCtrl,
    add: Button,
    remove: Button,
}

impl AddRemoveCtrl {
    pub fn new<W: Window>(parent: &W, frame: &Frame) -> Self {
        let list = ListBox::new(parent);
        let entry = TextCtrl::new(parent, "");
        let add = Button::new(parent, "Add");
        let remove = Button::new(parent, "Remove");
        let list_c = list.clone();
        let entry_c = entry.clone();
        let aid = add.id();
        let f = frame.clone();
        f.register_command_handler(aid, Box::new(move || {
            let text = entry_c.get_value();
            if !text.is_empty() {
                list_c.append(&text);
                entry_c.set_value("");
            }
        }));
        let list_c2 = list.clone();
        let rid = remove.id();
        let f2 = frame.clone();
        f2.register_command_handler(rid, Box::new(move || {
            if let Some(idx) = list_c2.get_selection() {
                list_c2.remove(idx);
            }
        }));
        Self {
            list,
            entry,
            add,
            remove,
        }
    }

    pub fn list_widget(&self) -> WidgetRef {
        self.list.as_widget_ref()
    }

    pub fn entry_widget(&self) -> WidgetRef {
        self.entry.as_widget_ref()
    }

    pub fn add_widget(&self) -> WidgetRef {
        self.add.as_widget_ref()
    }

    pub fn remove_widget(&self) -> WidgetRef {
        self.remove.as_widget_ref()
    }
}
