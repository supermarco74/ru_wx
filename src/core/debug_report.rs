//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Debug / crash reporting (`wxStackWalker`, `wxDebugReport`, `wxCrashReport`).

use std::fmt::Write as _;

/// Stack trace placeholder (`wxStackWalker`).
#[derive(Debug, Default)]
pub struct StackWalker {
    frames: Vec<String>,
}

impl StackWalker {
    pub fn capture() -> Self {
        Self {
            frames: vec!["(stack walk stub)".into()],
        }
    }

    pub fn frames(&self) -> &[String] {
        &self.frames
    }
}

/// Debug report bundle (`wxDebugReport`).
#[derive(Debug, Default)]
pub struct DebugReport {
    lines: Vec<String>,
}

impl DebugReport {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_text(&mut self, section: &str, text: &str) {
        self.lines.push(format!("[{section}] {text}"));
    }

    pub fn add_stack(&mut self) {
        for frame in StackWalker::capture().frames() {
            self.lines.push(format!("  at {frame}"));
        }
    }

    pub fn to_text(&self) -> String {
        let mut out = String::new();
        for line in &self.lines {
            let _ = writeln!(&mut out, "{line}");
        }
        out
    }
}

/// Crash report hook (`wxCrashReport`).
pub struct CrashReport;

impl CrashReport {
    pub fn install_hook() {
        // Platform-specific crash handlers would be registered here.
    }

    pub fn generate() -> DebugReport {
        let mut report = DebugReport::new();
        report.add_text("crash", "stub crash report");
        report.add_stack();
        report
    }
}
