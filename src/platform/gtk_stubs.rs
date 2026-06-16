//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! GTK backend stubs (Linux) — `wxFrame`, `wxPanel`, `wxButton`, `wxStaticText`, `wxApp`.

use std::cell::RefCell;
use std::rc::Rc;

use crate::core::geometry::Colour;
use crate::platform::stub_backend::{
    StubApp, StubBackend, StubButton, StubFrame, StubPanel, StubStaticText,
};

/// GTK application entry (`wxApp`).
#[derive(Debug)]
pub struct GtkApp {
    inner: StubApp,
}

impl Default for GtkApp {
    fn default() -> Self {
        Self::new()
    }
}

impl GtkApp {
    pub fn new() -> Self {
        Self {
            inner: StubApp::new(StubBackend::Gtk),
        }
    }

    pub fn run(&mut self) {
        self.inner.run();
    }

    pub fn is_running(&self) -> bool {
        self.inner.is_running()
    }

    pub fn stop(&mut self) {
        self.inner.stop();
    }

    pub fn backend(&self) -> StubBackend {
        self.inner.backend()
    }
}

/// GTK frame stub (`wxFrame`).
#[derive(Debug, Clone)]
pub struct GtkFrame {
    inner: Rc<RefCell<StubFrame>>,
}

impl GtkFrame {
    pub fn new(title: &str) -> Self {
        Self::with_size(title, 800, 600)
    }

    pub fn with_size(title: &str, width: u32, height: u32) -> Self {
        Self {
            inner: Rc::new(RefCell::new(StubFrame::new(
                StubBackend::Gtk,
                title,
                width,
                height,
            ))),
        }
    }

    pub fn handle(&self) -> isize {
        self.inner.borrow().handle
    }

    pub fn title(&self) -> String {
        self.inner.borrow().title.clone()
    }

    pub fn set_title(&self, title: &str) {
        self.inner.borrow_mut().set_title(title);
    }

    pub fn show(&self) {
        self.inner.borrow_mut().show();
    }

    pub fn register_command_handler(&self, id: u16, handler: Box<dyn FnMut()>) {
        self.inner.borrow().register_command_handler(id, handler);
    }

    pub fn dispatch_command(&self, id: u16) -> bool {
        self.inner.borrow().dispatch_command(id)
    }
}

/// GTK panel stub (`wxPanel`).
#[derive(Debug, Clone)]
pub struct GtkPanel {
    inner: Rc<RefCell<StubPanel>>,
}

impl GtkPanel {
    pub fn new(parent: &GtkFrame) -> Self {
        Self {
            inner: Rc::new(RefCell::new(StubPanel::new(
                StubBackend::Gtk,
                parent.handle(),
            ))),
        }
    }

    pub fn handle(&self) -> isize {
        self.inner.borrow().handle
    }

    pub fn set_background_colour(&self, colour: Colour) {
        self.inner.borrow_mut().set_background_colour(colour);
    }
}

/// GTK button stub (`wxButton`).
#[derive(Debug, Clone)]
pub struct GtkButton {
    inner: Rc<RefCell<StubButton>>,
}

impl GtkButton {
    pub fn new(parent: &GtkFrame, id: u16, label: &str) -> Self {
        Self {
            inner: Rc::new(RefCell::new(StubButton::new(
                StubBackend::Gtk,
                parent.handle(),
                id,
                label,
            ))),
        }
    }

    pub fn handle(&self) -> isize {
        self.inner.borrow().handle
    }

    pub fn id(&self) -> u16 {
        self.inner.borrow().id
    }

    pub fn label(&self) -> String {
        self.inner.borrow().label.to_string()
    }

    pub fn set_label(&self, label: &str) {
        self.inner.borrow_mut().set_label(label);
    }

    pub fn simulate_click(&self, frame: &GtkFrame) -> bool {
        let id = self.inner.borrow().id;
        frame.dispatch_command(id)
    }
}

/// GTK static text stub (`wxStaticText`).
#[derive(Debug, Clone)]
pub struct GtkStaticText {
    inner: Rc<RefCell<StubStaticText>>,
}

impl GtkStaticText {
    pub fn new(parent: &GtkFrame, label: &str) -> Self {
        Self {
            inner: Rc::new(RefCell::new(StubStaticText::new(
                StubBackend::Gtk,
                parent.handle(),
                label,
            ))),
        }
    }

    pub fn handle(&self) -> isize {
        self.inner.borrow().handle
    }

    pub fn label(&self) -> String {
        self.inner.borrow().label.to_string()
    }

    pub fn set_label(&self, label: &str) {
        self.inner.borrow_mut().set_label(label);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gtk_app_runs_and_stops() {
        let mut app = GtkApp::new();
        assert!(!app.is_running());
        app.run();
        assert!(app.is_running());
        app.stop();
        assert!(!app.is_running());
    }

    #[test]
    fn gtk_static_text_updates_label() {
        let frame = GtkFrame::new("GTK");
        let label = GtkStaticText::new(&frame, "one");
        label.set_label("two");
        assert_eq!(label.label(), "two");
    }
}
