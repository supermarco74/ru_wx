//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! GUI log sink (`wxLogGui`).

use std::sync::mpsc::Sender;
use std::sync::Arc;

use crate::adv::log_window::LogWindow;
use crate::core::log::{LogFormatter, LogRecord, LogTarget};

/// Thread-safe log target; drain with [`LogGuiTarget::drain_to`] on the UI thread.
pub struct LogGuiTarget {
    sender: Sender<String>,
    formatter: LogFormatter,
}

impl LogGuiTarget {
    pub fn new(sender: Sender<String>) -> Self {
        Self {
            sender,
            formatter: LogFormatter::new(),
        }
    }

    /// Wrap this target into the `Arc<dyn LogTarget>` expected by
    /// the logging facade.
    pub fn into_target(self) -> Arc<dyn LogTarget> {
        Arc::new(self)
    }

    /// Append queued lines to a [`LogWindow`] (call from `on_idle`).
    pub fn drain_to(receiver: &std::sync::mpsc::Receiver<String>, window: &LogWindow) {
        while let Ok(line) = receiver.try_recv() {
            window.append(&line);
        }
    }
}

impl LogTarget for LogGuiTarget {
    fn log_record(&self, record: &LogRecord) {
        let line = self.formatter.format(record);
        let _ = self.sender.send(line);
    }

    fn flush(&self) {}
}
