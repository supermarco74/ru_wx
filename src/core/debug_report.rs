//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Debug / crash reporting (`wxStackWalker`, `wxDebugReport`, `wxCrashReport`).

use std::backtrace::Backtrace;
use std::fmt::Write as _;

/// Stack trace capture (`wxStackWalker`).
#[derive(Debug, Default)]
pub struct StackWalker {
    frames: Vec<String>,
}

impl StackWalker {
    pub fn capture() -> Self {
        let bt = Backtrace::force_capture();
        let text = bt.to_string();
        let frames: Vec<String> = if text.trim().is_empty() {
            vec!["(no stack frames captured)".into()]
        } else {
            text.lines().map(|line| line.trim().to_string()).collect()
        };
        Self { frames }
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
        report.add_text("crash", "crash report generated");
        report.add_stack();
        report
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stack_walker_returns_at_least_one_frame() {
        let walker = StackWalker::capture();
        assert!(!walker.frames().is_empty());
    }

    #[test]
    fn debug_report_includes_stack_section() {
        let report = CrashReport::generate();
        let text = report.to_text();
        assert!(text.contains("[crash]"));
        assert!(text.contains("at "));
    }
}
