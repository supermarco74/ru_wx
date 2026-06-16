//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! AppKit backend stubs (macOS) — `wxFrame`, `wxPanel`, `wxButton`, `wxStaticText`, `wxApp`.

use std::cell::RefCell;
use std::rc::Rc;

use crate::core::geometry::Colour;
use crate::platform::stub_backend::{
    StubApp, StubBackend, StubButton, StubFrame, StubPanel, StubStaticText,
};

/// AppKit application entry (`wxApp`).
#[derive(Debug)]
pub struct AppKitApp {
    inner: StubApp,
}

impl Default for AppKitApp {
    fn default() -> Self {
        Self::new()
    }
}

impl AppKitApp {
    pub fn new() -> Self {
        Self {
            inner: StubApp::new(StubBackend::AppKit),
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

/// AppKit frame stub (`wxFrame`).
#[derive(Debug, Clone)]
pub struct AppKitFrame {
    inner: Rc<RefCell<StubFrame>>,
}

impl AppKitFrame {
    pub fn new(title: &str) -> Self {
        Self::with_size(title, 800, 600)
    }

    pub fn with_size(title: &str, width: u32, height: u32) -> Self {
        Self {
            inner: Rc::new(RefCell::new(StubFrame::new(
                StubBackend::AppKit,
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

/// AppKit panel stub (`wxPanel`).
#[derive(Debug, Clone)]
pub struct AppKitPanel {
    inner: Rc<RefCell<StubPanel>>,
}

impl AppKitPanel {
    pub fn new(parent: &AppKitFrame) -> Self {
        Self {
            inner: Rc::new(RefCell::new(StubPanel::new(
                StubBackend::AppKit,
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

/// AppKit button stub (`wxButton`).
#[derive(Debug, Clone)]
pub struct AppKitButton {
    inner: Rc<RefCell<StubButton>>,
}

impl AppKitButton {
    pub fn new(parent: &AppKitFrame, id: u16, label: &str) -> Self {
        Self {
            inner: Rc::new(RefCell::new(StubButton::new(
                StubBackend::AppKit,
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

    pub fn simulate_click(&self, frame: &AppKitFrame) -> bool {
        let id = self.inner.borrow().id;
        frame.dispatch_command(id)
    }
}

/// AppKit static text stub (`wxStaticText`).
#[derive(Debug, Clone)]
pub struct AppKitStaticText {
    inner: Rc<RefCell<StubStaticText>>,
}

impl AppKitStaticText {
    pub fn new(parent: &AppKitFrame, label: &str) -> Self {
        Self {
            inner: Rc::new(RefCell::new(StubStaticText::new(
                StubBackend::AppKit,
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
    use std::cell::Cell;
    use std::rc::Rc;

    #[test]
    fn appkit_app_runs_and_stops() {
        let mut app = AppKitApp::new();
        assert!(!app.is_running());
        app.run();
        assert!(app.is_running());
        app.stop();
        assert!(!app.is_running());
    }

    #[test]
    fn appkit_button_wires_to_frame_command() {
        let frame = AppKitFrame::new("Demo");
        let fired = Rc::new(Cell::new(false));
        let fired_cb = Rc::clone(&fired);
        frame.register_command_handler(7, Box::new(move || fired_cb.set(true)));
        let btn = AppKitButton::new(&frame, 7, "Click");
        assert!(btn.simulate_click(&frame));
        assert!(fired.get());
    }
}
