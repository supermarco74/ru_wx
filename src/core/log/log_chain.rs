//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Chained log target (`wxLogChain`).

use std::sync::Arc;

use super::target::{ChainTarget, LogTarget};

/// Chains two log targets (`wxLogChain`).
pub type LogChain = ChainTarget;

impl LogChain {
    pub fn chain(primary: Arc<dyn LogTarget>, secondary: Arc<dyn LogTarget>) -> Self {
        Self::new(primary, secondary)
    }
}
