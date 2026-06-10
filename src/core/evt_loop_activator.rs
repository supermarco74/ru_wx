//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Nested event-loop activator (`wxEvtLoopActivator`).

use crate::core::event_loop::EventLoop;

/// Temporarily marks an auxiliary loop as active (`wxEvtLoopActivator`).
pub struct EvtLoopActivator<'a> {
    loop_ref: &'a mut EventLoop,
    previous: bool,
}

impl<'a> EvtLoopActivator<'a> {
    pub fn new(loop_ref: &'a mut EventLoop) -> Self {
        let previous = loop_ref.is_running();
        Self { loop_ref, previous }
    }

    pub fn loop_ref(&self) -> &EventLoop {
        self.loop_ref
    }

    pub fn loop_mut(&mut self) -> &mut EventLoop {
        self.loop_ref
    }
}

impl Drop for EvtLoopActivator<'_> {
    fn drop(&mut self) {
        if !self.previous {
            self.loop_ref.dispatch_pending();
        }
    }
}

