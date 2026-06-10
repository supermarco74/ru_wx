//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! AppKit backend stubs (macOS) — `wxFrame`, `wxPanel`, `wxButton`, `wxStaticText`, `wxApp`.

/// AppKit application entry (`wxApp`).
#[derive(Debug, Default)]
pub struct AppKitApp {
    running: bool,
}

impl AppKitApp {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn run(&mut self) {
        self.running = true;
    }

    pub fn is_running(&self) -> bool {
        self.running
    }
}

/// AppKit frame stub (`wxFrame`).
#[derive(Debug)]
pub struct AppKitFrame {
    pub title: String,
}

impl AppKitFrame {
    pub fn new(title: &str) -> Self {
        Self {
            title: title.to_string(),
        }
    }
}

/// AppKit panel stub (`wxPanel`).
#[derive(Debug, Default)]
pub struct AppKitPanel;

/// AppKit button stub (`wxButton`).
#[derive(Debug)]
pub struct AppKitButton {
    pub label: String,
}

impl AppKitButton {
    pub fn new(label: &str) -> Self {
        Self {
            label: label.to_string(),
        }
    }
}

/// AppKit static text stub (`wxStaticText`).
#[derive(Debug)]
pub struct AppKitStaticText {
    pub label: String,
}

impl AppKitStaticText {
    pub fn new(label: &str) -> Self {
        Self {
            label: label.to_string(),
        }
    }
}
