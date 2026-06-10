//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Print-preview toolbar (`wxPreviewControlBar`).

use crate::chrome::tool_bar::ToolBar;
use crate::window::frame::Frame;

/// Navigation bar for print preview (`wxPreviewControlBar`).
pub struct PreviewControlBar {
    toolbar: ToolBar,
}

impl PreviewControlBar {
    pub fn new(frame: &Frame) -> Self {
        Self {
            toolbar: ToolBar::new(frame),
        }
    }

    pub fn add_navigation(&self, prev_id: u16, next_id: u16, close_id: u16) {
        self.toolbar.add_tool(prev_id, "Previous", 0);
        self.toolbar.add_tool(next_id, "Next", 0);
        self.toolbar.add_tool(close_id, "Close", 0);
        self.toolbar.realize();
    }

    pub fn on_action<F: FnMut(u16) + 'static>(&self, frame: &Frame, f: F) {
        self.toolbar.on_tool_clicked(frame, f);
    }
}
