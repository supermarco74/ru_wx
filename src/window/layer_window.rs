//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Layered popup window (`wxLayerWindow`).

use crate::core::geometry::Rect;
use crate::window::frame::Frame;
use crate::window::popup_window::PopupWindow;

/// Semi-transparent overlay window (`wxLayerWindow`).
#[derive(Clone)]
pub struct LayerWindow {
    popup: PopupWindow,
    opacity: u8,
}

impl LayerWindow {
    pub fn new(parent: &Frame, rect: Rect) -> Self {
        Self {
            popup: PopupWindow::new(parent, rect),
            opacity: 255,
        }
    }

    pub fn set_opacity(&mut self, opacity: u8) {
        self.opacity = opacity;
    }

    pub fn opacity(&self) -> u8 {
        self.opacity
    }

    pub fn close(&self) {
        self.popup.close();
    }

    #[cfg(target_os = "windows")]
    pub fn hwnd(&self) -> windows_sys::Win32::Foundation::HWND {
        self.popup.hwnd()
    }
}
