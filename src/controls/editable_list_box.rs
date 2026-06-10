//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! List with inline editing (`wxEditableListBox`).

use crate::controls::list_box::ListBox;
use crate::controls::text_ctrl::TextCtrl;
use crate::core::widget::{WidgetRef, Window};
use crate::window::frame::Frame;

#[derive(Clone)]
pub struct EditableListBox {
    list: ListBox,
    editor: TextCtrl,
}

impl EditableListBox {
    pub fn new<W: Window>(parent: &W) -> Self {
        Self {
            list: ListBox::new(parent),
            editor: TextCtrl::new(parent, ""),
        }
    }

    pub fn append(&self, item: &str) {
        self.list.append(item);
    }

    pub fn selection(&self) -> Option<usize> {
        self.list.get_selection()
    }

    pub fn load_selection_into_editor(&self) {
        if let Some(idx) = self.list.get_selection() {
            if let Some(text) = self.list.get_string(idx) {
                self.editor.set_value(&text);
            }
        }
    }

    pub fn commit_editor(&self) {
        if let Some(idx) = self.list.get_selection() {
            let text = self.editor.get_value();
            self.list.remove(idx);
            self.list.insert(idx, &text);
            self.list.set_selection(idx);
        }
    }

    pub fn bind_selection_to_editor(&self, frame: &Frame) {
        let this = self.clone();
        self.list.on_selection_change(frame, move || {
            this.load_selection_into_editor();
        });
    }

    pub fn list_widget(&self) -> WidgetRef {
        self.list.as_widget_ref()
    }

    pub fn editor_widget(&self) -> WidgetRef {
        self.editor.as_widget_ref()
    }
}
