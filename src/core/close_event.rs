//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Close-event helper (`wxCloseEvent` veto support).
//!
//! Register on a [`crate::Frame`] with [`crate::Frame::on_query_close`].
//! Call [`CloseEvent::veto`] inside the handler to cancel window close.

use std::cell::Cell;

/// Delivered when the user tries to close a top-level window.
#[derive(Debug)]
pub struct CloseEvent {
    vetoed: Cell<bool>,
}

impl CloseEvent {
    /// Start a new close query (not vetoed).
    pub(crate) fn new() -> Self {
        Self {
            vetoed: Cell::new(false),
        }
    }

    /// Prevent the window from closing (wx `Veto()`).
    pub fn veto(&self) {
        self.vetoed.set(true);
    }

    /// Whether the handler called [`Self::veto`].
    pub fn is_vetoed(&self) -> bool {
        self.vetoed.get()
    }
}
