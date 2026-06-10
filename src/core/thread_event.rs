//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Cross-thread events (`wxThreadEvent`).

/// Event posted from a worker thread (`wxThreadEvent`).
#[derive(Debug, Clone)]
pub struct ThreadEvent {
    pub event_type: u32,
    pub payload: String,
}

impl ThreadEvent {
    pub fn new(event_type: u32, payload: impl Into<String>) -> Self {
        Self {
            event_type,
            payload: payload.into(),
        }
    }
}
