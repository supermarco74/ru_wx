//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Print-preview frame (`wxPreviewFrame`).

use crate::printing::preview_control_bar::PreviewControlBar;
use crate::printing::{PrintPreview, Printout};
use crate::window::frame::Frame;

/// Host window for print preview (`wxPreviewFrame`).
pub struct PreviewFrame {
    frame: Frame,
    preview: PrintPreview,
    control_bar: PreviewControlBar,
}

impl PreviewFrame {
    pub fn new(title: &str, printout: Printout) -> Self {
        let frame = Frame::builder()
            .with_title(title)
            .with_size(640, 480)
            .build();
        let preview = PrintPreview::new(printout);
        let control_bar = PreviewControlBar::new(&frame);
        Self {
            frame,
            preview,
            control_bar,
        }
    }

    pub fn frame(&self) -> &Frame {
        &self.frame
    }

    pub fn preview(&self) -> &PrintPreview {
        &self.preview
    }

    pub fn preview_mut(&mut self) -> &mut PrintPreview {
        &mut self.preview
    }

    pub fn control_bar(&self) -> &PreviewControlBar {
        &self.control_bar
    }
}
