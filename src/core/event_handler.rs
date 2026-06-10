//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Event-handler base (`wxEvtHandler`).

use crate::core::command_event::CommandEvent;
use crate::window::frame::Frame;

/// Base trait for objects that receive UI events (`wxEvtHandler`).
pub trait EvtHandler {
    fn bind_command<F>(&self, frame: &Frame, id: u16, f: F)
    where
        F: FnMut(&CommandEvent) + 'static;
}

/// Blanket helper for any type that can register Win32 command ids.
pub struct CommandBinder;

impl CommandBinder {
    pub fn bind<F>(frame: &Frame, id: u16, mut f: F)
    where
        F: FnMut(&CommandEvent) + 'static,
    {
        frame.register_command_handler(
            id,
            Box::new(move || f(&CommandEvent::new(id))),
        );
    }
}
