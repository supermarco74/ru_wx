//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Advanced UI docking (`wxAuiManager` family).

use crate::chrome::aui_tool_bar::AuiDockSide;
use crate::core::geometry::Rect;
use crate::window::frame::Frame;

/// Pane metadata (`wxAuiPaneInfo`).
#[derive(Debug, Clone)]
pub struct AuiPaneInfo {
    pub name: String,
    pub caption: String,
    pub dock_side: AuiDockSide,
    pub rect: Rect,
    pub floating: bool,
    pub shown: bool,
}

impl AuiPaneInfo {
    pub fn toolbar(name: &str) -> Self {
        Self {
            name: name.to_string(),
            caption: name.to_string(),
            dock_side: AuiDockSide::Top,
            rect: Rect::new(0, 0, 0, 32),
            floating: false,
            shown: true,
        }
    }

    pub fn with_caption(mut self, caption: &str) -> Self {
        self.caption = caption.to_string();
        self
    }
}

/// Dock art / theme hooks (`wxAuiDockArt`).
#[derive(Debug, Default)]
pub struct AuiDockArt {
    sash_width: u32,
}

impl AuiDockArt {
    pub fn new() -> Self {
        Self { sash_width: 5 }
    }

    pub fn sash_width(&self) -> u32 {
        self.sash_width
    }
}

/// Floating frame placeholder (`wxAuiFloatingFrame`).
pub struct AuiFloatingFrame {
    title: String,
    rect: Rect,
}

impl AuiFloatingFrame {
    pub fn new(title: &str, rect: Rect) -> Self {
        Self {
            title: title.to_string(),
            rect,
        }
    }
}

/// Tabbed dock notebook (`wxAuiNotebook`).
#[derive(Default)]
pub struct AuiNotebook {
    pages: Vec<String>,
    selection: usize,
}

impl AuiNotebook {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_page(&mut self, caption: &str) -> usize {
        self.pages.push(caption.to_string());
        self.pages.len() - 1
    }

    pub fn set_selection(&mut self, index: usize) {
        if index < self.pages.len() {
            self.selection = index;
        }
    }

    pub fn selection(&self) -> usize {
        self.selection
    }
}

/// Main docking manager (`wxAuiManager`).
pub struct AuiManager {
    panes: Vec<AuiPaneInfo>,
    art: AuiDockArt,
}

impl Default for AuiManager {
    fn default() -> Self {
        Self::new()
    }
}

impl AuiManager {
    pub fn new() -> Self {
        Self {
            panes: Vec::new(),
            art: AuiDockArt::new(),
        }
    }

    pub fn add_pane(&mut self, info: AuiPaneInfo) {
        self.panes.push(info);
    }

    pub fn update(&self, _frame: &Frame) {
        // Layout refresh hook — panes are positioned by the frame sizer in minitests.
    }

    pub fn pane_count(&self) -> usize {
        self.panes.len()
    }

    pub fn dock_art(&self) -> &AuiDockArt {
        &self.art
    }
}
