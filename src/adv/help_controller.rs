//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Context help controller (`wxHelpController`).

use crate::core::context_help_event::ContextHelpEvent;
use crate::core::geometry::Point;
use crate::window::frame::Frame;

/// Simple help routing (`wxHelpController`).
pub struct HelpController {
    topic: String,
}

impl HelpController {
    pub fn new(topic: &str) -> Self {
        Self {
            topic: topic.to_string(),
        }
    }

    pub fn display_contents(&self) -> &str {
        &self.topic
    }

    pub fn display_context(&self, event: &ContextHelpEvent) -> String {
        format!(
            "Help for control {} at ({}, {})",
            event.control_id, event.position.x, event.position.y
        )
    }

    pub fn wire_f1<F: FnMut(&ContextHelpEvent) + 'static>(&self, frame: &Frame, mut f: F) {
        frame.on_key_down(move |ev| {
            if ev.key_code == 0x70 {
                f(&ContextHelpEvent::new(0, Point::new(0, 0)));
            }
        });
    }
}
