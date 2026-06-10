//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! In-memory log buffer (`wxLogBuffer`).

use super::target::BufferTarget;

/// Buffers log records in memory (`wxLogBuffer`).
pub type LogBuffer = BufferTarget;

impl LogBuffer {
    pub fn messages(&self) -> Vec<String> {
        self.get_messages()
    }
}
