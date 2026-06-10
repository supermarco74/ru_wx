//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Timer tick event (`wxTimerEvent`).

/// Timer fired (`wxTimerEvent`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimerEvent {
    pub timer_id: u32,
}

impl TimerEvent {
    pub const fn new(timer_id: u32) -> Self {
        Self { timer_id }
    }
}
