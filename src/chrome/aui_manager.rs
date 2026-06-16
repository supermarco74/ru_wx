//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Advanced UI docking (`wxAuiManager` family).

use crate::chrome::aui_tool_bar::AuiDockSide;
use crate::core::geometry::Rect;
use crate::core::widget::Widget;
use crate::window::frame::Frame;

#[cfg(target_os = "windows")]
use windows_sys::Win32::Foundation::RECT;
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::WindowsAndMessaging::{GetClientRect, MoveWindow};

/// Pane metadata (`wxAuiPaneInfo`).
#[derive(Debug, Clone)]
pub struct AuiPaneInfo {
    pub name: String,
    pub caption: String,
    pub dock_side: AuiDockSide,
    pub rect: Rect,
    pub floating: bool,
    pub shown: bool,
    /// Native window handle (`Widget::native_handle`) positioned by [`AuiManager::update`].
    pub window_handle: Option<isize>,
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
            window_handle: None,
        }
    }

    pub fn with_caption(mut self, caption: &str) -> Self {
        self.caption = caption.to_string();
        self
    }

    pub fn with_window(mut self, widget: &dyn Widget) -> Self {
        self.window_handle = Some(widget.native_handle());
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

    /// Position registered pane HWNDs inside `frame`'s client area.
    pub fn update(&self, frame: &Frame) {
        #[cfg(target_os = "windows")]
        {
            let hwnd = frame.hwnd();
            if hwnd.is_null() {
                return;
            }
            let mut client = RECT {
                left: 0,
                top: 0,
                right: 0,
                bottom: 0,
            };
            // SAFETY: client rect query on a live frame HWND.
            unsafe {
                GetClientRect(hwnd, &mut client);
            }
            let cw = client.right - client.left;
            let ch = client.bottom - client.top;
            let sash = self.art.sash_width() as i32;

            let mut top_y = 0i32;
            for pane in self.panes.iter().filter(|p| {
                p.shown && !p.floating && p.dock_side == AuiDockSide::Top
            }) {
                let h = pane.rect.height as i32;
                if let Some(child) = pane.window_handle.filter(|&h| h != 0) {
                    // SAFETY: MoveWindow on a registered child HWND.
                    unsafe {
                        MoveWindow(child as _, 0, top_y, cw, h, 1);
                    }
                }
                top_y += h + sash;
            }

            let mut bottom_y = ch;
            for pane in self.panes.iter().filter(|p| {
                p.shown && !p.floating && p.dock_side == AuiDockSide::Bottom
            }) {
                let h = pane.rect.height as i32;
                bottom_y -= h;
                if let Some(child) = pane.window_handle.filter(|&h| h != 0) {
                    unsafe {
                        MoveWindow(child as _, 0, bottom_y, cw, h, 1);
                    }
                }
                bottom_y -= sash;
            }

            let center_h = (bottom_y - top_y).max(0);
            let mut left_x = 0i32;
            for pane in self.panes.iter().filter(|p| {
                p.shown && !p.floating && p.dock_side == AuiDockSide::Left
            }) {
                let w = if pane.rect.width > 0 {
                    pane.rect.width as i32
                } else {
                    200
                };
                if let Some(child) = pane.window_handle.filter(|&h| h != 0) {
                    unsafe {
                        MoveWindow(child as _, left_x, top_y, w, center_h, 1);
                    }
                }
                left_x += w + sash;
            }

            let mut right_x = cw;
            for pane in self.panes.iter().filter(|p| {
                p.shown && !p.floating && p.dock_side == AuiDockSide::Right
            }) {
                let w = if pane.rect.width > 0 {
                    pane.rect.width as i32
                } else {
                    200
                };
                right_x -= w;
                if let Some(child) = pane.window_handle.filter(|&h| h != 0) {
                    unsafe {
                        MoveWindow(child as _, right_x, top_y, w, center_h, 1);
                    }
                }
                right_x -= sash;
            }
        }
        #[cfg(not(target_os = "windows"))]
        {
            let _ = frame;
        }
    }

    pub fn pane_count(&self) -> usize {
        self.panes.len()
    }

    pub fn dock_art(&self) -> &AuiDockArt {
        &self.art
    }
}
