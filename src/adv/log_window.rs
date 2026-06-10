//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Log output window (`wxLogWindow`).

use crate::controls::list_box::ListBox;
use crate::core::widget::{WidgetRef, Window};
use crate::window::frame::Frame;

#[derive(Clone)]
pub struct LogWindow {
    list: ListBox,
}

impl LogWindow {
    pub fn new<W: Window>(parent: &W) -> Self {
        Self {
            list: ListBox::new(parent),
        }
    }

    pub fn append(&self, line: &str) {
        self.list.append(line);
    }

    pub fn as_widget_ref(&self) -> WidgetRef {
        self.list.as_widget_ref()
    }
}

/// Attach log lines to a frame-owned list.
pub fn attach_log_window(frame: &Frame, log: &LogWindow) {
    frame.add_widget(log.as_widget_ref());
}
