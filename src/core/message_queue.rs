//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Cross-thread message queue (`wxMessageQueue`).

use std::collections::VecDeque;
use std::sync::{Condvar, Mutex};

/// Thread-safe FIFO queue (`wxMessageQueue<T>`).
pub struct MessageQueue<T> {
    inner: Mutex<VecDeque<T>>,
    notify: Condvar,
}

impl<T> MessageQueue<T> {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(VecDeque::new()),
            notify: Condvar::new(),
        }
    }

    pub fn post(&self, item: T) {
        let mut q = self.inner.lock().expect("queue lock");
        q.push_back(item);
        self.notify.notify_one();
    }

    pub fn receive_timeout(&self, timeout_ms: u32) -> Option<T> {
        let mut q = self.inner.lock().expect("queue lock");
        if let Some(item) = q.pop_front() {
            return Some(item);
        }
        let (mut q, _) = self
            .notify
            .wait_timeout(q, std::time::Duration::from_millis(timeout_ms as u64))
            .expect("queue wait");
        q.pop_front()
    }

    pub fn try_receive(&self) -> Option<T> {
        self.inner.lock().expect("queue lock").pop_front()
    }
}

impl<T> Default for MessageQueue<T> {
    fn default() -> Self {
        Self::new()
    }
}
