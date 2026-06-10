//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! GTK backend stubs (Linux) — `wxFrame`, `wxPanel`, `wxButton`, `wxStaticText`, `wxApp`.

/// GTK application entry (`wxApp`).
#[derive(Debug, Default)]
pub struct GtkApp {
    running: bool,
}

impl GtkApp {
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

/// GTK frame stub (`wxFrame`).
#[derive(Debug)]
pub struct GtkFrame {
    pub title: String,
}

impl GtkFrame {
    pub fn new(title: &str) -> Self {
        Self {
            title: title.to_string(),
        }
    }
}

/// GTK panel stub (`wxPanel`).
#[derive(Debug, Default)]
pub struct GtkPanel;

/// GTK button stub (`wxButton`).
#[derive(Debug)]
pub struct GtkButton {
    pub label: String,
}

impl GtkButton {
    pub fn new(label: &str) -> Self {
        Self {
            label: label.to_string(),
        }
    }
}

/// GTK static text stub (`wxStaticText`).
#[derive(Debug)]
pub struct GtkStaticText {
    pub label: String,
}

impl GtkStaticText {
    pub fn new(label: &str) -> Self {
        Self {
            label: label.to_string(),
        }
    }
}
