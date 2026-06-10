//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Pre-dispatch event filter (`wxEventFilter`).

/// Intercepts events before they reach handlers (`wxEventFilter`).
pub trait EventFilter: Send {
    fn filter_event(&mut self, event_type: u32, source_id: u16) -> bool;
}

/// Accept every event (no-op filter).
#[derive(Debug, Default)]
pub struct PassThroughFilter;

impl EventFilter for PassThroughFilter {
    fn filter_event(&mut self, _event_type: u32, _source_id: u16) -> bool {
        true
    }
}

/// Drop events whose type id is listed.
#[derive(Debug, Default)]
pub struct BlockListFilter {
    blocked: Vec<u32>,
}

impl BlockListFilter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn block(&mut self, event_type: u32) {
        self.blocked.push(event_type);
    }
}

impl EventFilter for BlockListFilter {
    fn filter_event(&mut self, event_type: u32, _source_id: u16) -> bool {
        !self.blocked.contains(&event_type)
    }
}
