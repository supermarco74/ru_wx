//! XamlFrame — top-level window descriptor and builder.
//!
//! Mirrors the `ru_wx::Frame` API but targets the reactor backend.
//! The frame collects widget descriptors during the imperative phase;
//! `XamlApp::run()` converts them into a reactor element tree.

use std::cell::RefCell;
use std::rc::Rc;

use super::element_bridge::{FrameDesc, FrameRef};

/// Top-level application window.
///
/// Wraps a [`FrameDesc`] shared between the frame and all child widgets.
/// Widgets append themselves to `desc.children` when constructed with
/// `&frame` as parent.
pub struct XamlFrame {
    pub(crate) desc: FrameRef,
    /// Widget ID assigned to this frame (used for close/resize events).
    pub(crate) frame_id: u64,
}

impl XamlFrame {
    /// Start building a new frame.
    pub fn builder() -> XamlFrameBuilder {
        XamlFrameBuilder::new()
    }

    /// Get the frame's widget ID.
    pub fn id(&self) -> u64 {
        self.frame_id
    }

    /// Access the shared descriptor (used by widget constructors).
    pub(crate) fn desc_ref(&self) -> FrameRef {
        Rc::clone(&self.desc)
    }

    /// Register a close handler (called when the user clicks X).
    pub fn on_close<F>(&self, f: F)
    where
        F: FnMut() + Send + 'static,
    {
        let desc = self.desc.borrow();
        if let Ok(mut reg) = desc.registry.lock() {
            reg.close_handlers.insert(self.frame_id, Box::new(f));
        };
    }

    /// Register a resize handler.
    pub fn on_resize<F>(&self, f: F)
    where
        F: FnMut(i32, i32) + Send + 'static,
    {
        let desc = self.desc.borrow();
        if let Ok(mut reg) = desc.registry.lock() {
            reg.resize_handlers.insert(self.frame_id, Box::new(f));
        };
    }

    /// Set the window title at runtime.
    pub fn set_title(&self, title: &str) {
        self.desc.borrow_mut().title = title.to_string();
    }

    /// Get the current window title.
    pub fn title(&self) -> String {
        self.desc.borrow().title.clone()
    }

    /// Get the current window size as `(width, height)`.
    pub fn size(&self) -> (f64, f64) {
        let d = self.desc.borrow();
        (d.width, d.height)
    }

    /// Center the window on screen.
    pub fn center(&self) {
        self.desc.borrow_mut().centered = true;
    }

    /// Show the window.
    pub fn show(&self) {
        self.desc.borrow_mut().visible = true;
    }

    /// Hide the window.
    pub fn hide(&self) {
        self.desc.borrow_mut().visible = false;
    }
}

impl Clone for XamlFrame {
    fn clone(&self) -> Self {
        Self {
            desc: Rc::clone(&self.desc),
            frame_id: self.frame_id,
        }
    }
}

// ── Builder ──────────────────────────────────────────────────────────────────

/// Builder for [`XamlFrame`], mirroring `ru_wx::Frame::builder()`.
pub struct XamlFrameBuilder {
    title: String,
    width: f64,
    height: f64,
    centered: bool,
    padding: f64,
    spacing: f64,
}

impl XamlFrameBuilder {
    pub fn new() -> Self {
        Self {
            title: String::from("ru_wx App"),
            width: 800.0,
            height: 600.0,
            centered: false,
            padding: 0.0,
            spacing: 0.0,
        }
    }

    /// Set the window title.
    pub fn with_title(mut self, title: &str) -> Self {
        self.title = title.to_string();
        self
    }

    /// Set the window size in logical pixels.
    pub fn with_size(mut self, width: i32, height: i32) -> Self {
        self.width = width as f64;
        self.height = height as f64;
        self
    }

    /// Padding su tutti i lati del contenuto della finestra (px logici).
    pub fn with_padding(mut self, padding: f64) -> Self {
        self.padding = padding;
        self
    }

    /// Spaziatura verticale tra i widget di primo livello (px logici).
    pub fn with_spacing(mut self, spacing: f64) -> Self {
        self.spacing = spacing;
        self
    }

    /// Center the window on screen.
    pub fn centered(mut self) -> Self {
        self.centered = true;
        self
    }

    /// Build the frame descriptor.
    pub fn build(self) -> XamlFrame {
        let frame_id = super::element_bridge::next_id();
        let mut desc = FrameDesc::new();
        desc.title = self.title;
        desc.width = self.width;
        desc.height = self.height;
        desc.centered = self.centered;
        desc.padding = self.padding;
        desc.spacing = self.spacing;
        XamlFrame {
            desc: Rc::new(RefCell::new(desc)),
            frame_id,
        }
    }
}

impl Default for XamlFrameBuilder {
    fn default() -> Self {
        Self::new()
    }
}
