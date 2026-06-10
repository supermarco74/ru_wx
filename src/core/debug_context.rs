//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Debug context (`wxDebugContext`).

use crate::core::debug_report::DebugReport;

/// Runtime debug checks (`wxDebugContext`).
#[derive(Debug, Default)]
pub struct DebugContext {
    asserts_enabled: bool,
}

impl DebugContext {
    pub fn new() -> Self {
        Self {
            asserts_enabled: true,
        }
    }

    pub fn set_asserts_enabled(&mut self, enabled: bool) {
        self.asserts_enabled = enabled;
    }

    pub fn assert_msg(&self, condition: bool, message: &str) -> bool {
        if self.asserts_enabled && !condition {
            let mut report = DebugReport::new();
            report.add_text("assert", message);
            let _ = report.to_text();
            return false;
        }
        condition
    }

    pub fn check_memory_leaks(&self) -> bool {
        true
    }
}
