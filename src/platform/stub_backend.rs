//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Shared in-memory UI backend used by the AppKit and GTK placeholder modules.
//! Models a minimal widget tree with fake native handles, layout state, and a
//! stub event loop sufficient for unit tests and headless CI on non-Windows targets.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::atomic::{AtomicIsize, Ordering};

use crate::core::geometry::{Colour, Rect};

/// Which placeholder native toolkit owns a stub widget.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StubBackend {
    AppKit,
    Gtk,
}

static NEXT_STUB_HANDLE: AtomicIsize = AtomicIsize::new(0x1_0000);

fn next_stub_handle() -> isize {
    NEXT_STUB_HANDLE.fetch_add(1, Ordering::Relaxed)
}

/// Allocate a fake native handle for a stub widget (AppKit / GTK placeholders).
pub fn alloc_widget_handle() -> isize {
    next_stub_handle()
}

/// Application entry (`wxApp`) — stub event loop.
#[derive(Debug)]
pub struct StubApp {
    backend: StubBackend,
    running: bool,
    tick_count: u32,
}

impl StubApp {
    pub fn new(backend: StubBackend) -> Self {
        Self {
            backend,
            running: false,
            tick_count: 0,
        }
    }

    pub fn backend(&self) -> StubBackend {
        self.backend
    }

    pub fn is_running(&self) -> bool {
        self.running
    }

    /// Mark the application as running (stub — no native loop yet).
    pub fn run(&mut self) {
        self.running = true;
    }

    /// Run a bounded stub loop that invokes `on_tick` until `should_continue`
    /// returns false or `max_ticks` is reached.
    pub fn run_with<F, G>(&mut self, max_ticks: u32, mut on_tick: F, mut should_continue: G)
    where
        F: FnMut(u32),
        G: FnMut() -> bool,
    {
        self.running = true;
        self.tick_count = 0;
        while self.running && self.tick_count < max_ticks && should_continue() {
            on_tick(self.tick_count);
            self.tick_count += 1;
        }
        self.running = false;
    }

    pub fn stop(&mut self) {
        self.running = false;
    }

    pub fn tick_count(&self) -> u32 {
        self.tick_count
    }
}

impl Default for StubApp {
    fn default() -> Self {
        Self::new(StubBackend::Gtk)
    }
}

type CommandHandlers = Rc<RefCell<HashMap<u16, Box<dyn FnMut()>>>>;

fn fresh_handlers() -> CommandHandlers {
    Rc::new(RefCell::new(HashMap::new()))
}

/// Top-level window (`wxFrame`).
#[derive(Clone)]
pub struct StubFrame {
    pub handle: isize,
    pub backend: StubBackend,
    pub title: String,
    pub rect: Rect,
    pub visible: bool,
    command_handlers: CommandHandlers,
}

impl std::fmt::Debug for StubFrame {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StubFrame")
            .field("handle", &self.handle)
            .field("backend", &self.backend)
            .field("title", &self.title)
            .field("rect", &self.rect)
            .field("visible", &self.visible)
            .field(
                "command_handler_count",
                &self.command_handlers.borrow().len(),
            )
            .finish()
    }
}

impl StubFrame {
    pub fn new(backend: StubBackend, title: &str, width: u32, height: u32) -> Self {
        Self {
            handle: next_stub_handle(),
            backend,
            title: title.to_string(),
            rect: Rect::new(0, 0, width, height),
            visible: false,
            command_handlers: fresh_handlers(),
        }
    }

    pub fn set_title(&mut self, title: &str) {
        self.title = title.to_string();
    }

    pub fn show(&mut self) {
        self.visible = true;
    }

    pub fn register_command_handler(&self, id: u16, handler: Box<dyn FnMut()>) {
        self.command_handlers.borrow_mut().insert(id, handler);
    }

    pub fn dispatch_command(&self, id: u16) -> bool {
        let handler = self.command_handlers.borrow_mut().remove(&id);
        if let Some(mut handler) = handler {
            handler();
            self.command_handlers.borrow_mut().insert(id, handler);
            true
        } else {
            false
        }
    }

    pub fn command_handler_count(&self) -> usize {
        self.command_handlers.borrow().len()
    }
}

/// Container panel (`wxPanel`).
#[derive(Debug, Clone)]
pub struct StubPanel {
    pub handle: isize,
    pub backend: StubBackend,
    pub parent: isize,
    pub rect: Rect,
    pub background_colour: Colour,
    pub visible: bool,
    pub enabled: bool,
}

impl StubPanel {
    pub fn new(backend: StubBackend, parent: isize) -> Self {
        Self {
            handle: next_stub_handle(),
            backend,
            parent,
            rect: Rect::new(0, 0, 200, 200),
            background_colour: Colour::LIGHT_GREY,
            visible: true,
            enabled: true,
        }
    }

    pub fn set_background_colour(&mut self, colour: Colour) {
        self.background_colour = colour;
    }
}

/// Push button (`wxButton`).
#[derive(Debug, Clone)]
pub struct StubButton {
    pub handle: isize,
    pub backend: StubBackend,
    pub parent: isize,
    pub id: u16,
    pub label: String,
    pub rect: Rect,
    pub enabled: bool,
    pub visible: bool,
    pub flat: bool,
}

impl StubButton {
    pub fn new(backend: StubBackend, parent: isize, id: u16, label: &str) -> Self {
        Self {
            handle: next_stub_handle(),
            backend,
            parent,
            id,
            label: label.to_string(),
            rect: Rect::new(0, 0, 100, 30),
            enabled: true,
            visible: true,
            flat: false,
        }
    }

    pub fn set_label(&mut self, label: &str) {
        self.label = label.to_string();
    }

    pub fn label(&self) -> &str {
        &self.label
    }

    pub fn simulate_click(&self, frame: &StubFrame) -> bool {
        frame.dispatch_command(self.id)
    }
}

/// Read-only label (`wxStaticText`).
#[derive(Debug, Clone)]
pub struct StubStaticText {
    pub handle: isize,
    pub backend: StubBackend,
    pub parent: isize,
    pub label: String,
    pub rect: Rect,
    pub visible: bool,
}

impl StubStaticText {
    pub fn new(backend: StubBackend, parent: isize, label: &str) -> Self {
        Self {
            handle: next_stub_handle(),
            backend,
            parent,
            label: label.to_string(),
            rect: Rect::new(0, 0, 200, 20),
            visible: true,
        }
    }

    pub fn set_label(&mut self, label: &str) {
        self.label = label.to_string();
    }

    pub fn label(&self) -> &str {
        &self.label
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    use std::rc::Rc;

    #[test]
    fn stub_button_click_dispatches_command() {
        let frame = StubFrame::new(StubBackend::AppKit, "Test", 400, 300);
        let clicked = Rc::new(Cell::new(false));
        let clicked_cb = Rc::clone(&clicked);
        frame.register_command_handler(42, Box::new(move || clicked_cb.set(true)));
        let btn = StubButton::new(StubBackend::AppKit, frame.handle, 42, "OK");
        assert!(btn.simulate_click(&frame));
        assert!(clicked.get());
    }

    #[test]
    fn stub_app_bounded_loop_runs_ticks() {
        let mut app = StubApp::new(StubBackend::Gtk);
        let ticks = Cell::new(0_u32);
        app.run_with(
            5,
            |_| ticks.set(ticks.get() + 1),
            || ticks.get() < 3,
        );
        assert_eq!(ticks.get(), 3);
        assert!(!app.is_running());
    }

    #[test]
    fn stub_static_text_label_round_trip() {
        let mut label = StubStaticText::new(StubBackend::Gtk, 1, "hello");
        label.set_label("world");
        assert_eq!(label.label(), "world");
    }
}
