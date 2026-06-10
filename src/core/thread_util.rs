//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Threading helpers (`wxThread`, `wxMutex`).

use std::sync::{Arc, Mutex, MutexGuard};

/// Shared mutex wrapper (`wxMutex`).
pub struct WxMutex<T> {
    inner: Mutex<T>,
}

impl<T> WxMutex<T> {
    pub fn new(value: T) -> Self {
        Self {
            inner: Mutex::new(value),
        }
    }

    pub fn lock(&self) -> MutexGuard<'_, T> {
        self.inner.lock().expect("mutex poisoned")
    }
}

/// Background worker (`wxThread`).
pub struct WxThread {
    handle: Option<std::thread::JoinHandle<()>>,
}

impl WxThread {
    pub fn spawn<F>(f: F) -> Self
    where
        F: FnOnce() + Send + 'static,
    {
        Self {
            handle: Some(std::thread::spawn(f)),
        }
    }

    pub fn join(mut self) {
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
    }
}

/// Shareable flag for cross-thread UI signals.
pub type SharedFlag = Arc<WxMutex<bool>>;
