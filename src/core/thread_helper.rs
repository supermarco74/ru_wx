//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Thread helper (`wxThreadHelper`).

use std::sync::mpsc::{Receiver, Sender};
use std::thread::JoinHandle;

use crate::core::thread_util::WxThread;

/// Background worker with completion channel (`wxThreadHelper`).
pub struct ThreadHelper<T: Send + 'static> {
    handle: Option<JoinHandle<()>>,
    receiver: Receiver<T>,
}

impl<T: Send + 'static> ThreadHelper<T> {
    pub fn spawn<F>(f: F) -> (Self, Sender<T>)
    where
        F: FnOnce(Sender<T>) + Send + 'static,
    {
        let (tx, rx) = std::sync::mpsc::channel();
        let tx_worker = tx.clone();
        let handle = std::thread::spawn(move || f(tx_worker));
        (
            Self {
                handle: Some(handle),
                receiver: rx,
            },
            tx,
        )
    }

    pub fn try_receive(&self) -> Option<T> {
        self.receiver.try_recv().ok()
    }

    pub fn join(mut self) {
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
    }
}

/// Convenience wrapper around [`WxThread`].
pub struct ThreadHelperSimple {
    thread: WxThread,
}

impl ThreadHelperSimple {
    pub fn run<F>(f: F) -> Self
    where
        F: FnOnce() + Send + 'static,
    {
        Self {
            thread: WxThread::spawn(f),
        }
    }

    pub fn join(self) {
        self.thread.join();
    }
}
