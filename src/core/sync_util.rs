//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Extra synchronisation (`wxCondition`, `wxSemaphore`, `wxCriticalSection`).

use std::sync::{Condvar, Mutex, MutexGuard};

/// Condition variable wrapper (`wxCondition`).
pub struct WxCondition {
    pair: (Mutex<bool>, Condvar),
}

impl WxCondition {
    pub fn new() -> Self {
        Self {
            pair: (Mutex::new(false), Condvar::new()),
        }
    }

    pub fn wait(&self) {
        let mut ready = self.pair.0.lock().expect("cond lock");
        while !*ready {
            ready = self.pair.1.wait(ready).expect("cond wait");
        }
        *ready = false;
    }

    pub fn signal(&self) {
        let mut ready = self.pair.0.lock().expect("cond lock");
        *ready = true;
        self.pair.1.notify_one();
    }
}

impl Default for WxCondition {
    fn default() -> Self {
        Self::new()
    }
}

/// Counting semaphore (`wxSemaphore`).
pub struct WxSemaphore {
    count: Mutex<usize>,
    notify: Condvar,
    max: usize,
}

/// Critical section (`wxCriticalSection`).
pub struct WxCriticalSection {
    inner: Mutex<()>,
}

impl WxCriticalSection {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(()),
        }
    }

    pub fn enter(&self) -> MutexGuard<'_, ()> {
        self.inner.lock().expect("critical section lock")
    }
}

impl Default for WxCriticalSection {
    fn default() -> Self {
        Self::new()
    }
}

impl WxSemaphore {
    pub fn new(max: usize) -> Self {
        Self {
            count: Mutex::new(max),
            notify: Condvar::new(),
            max,
        }
    }

    pub fn wait(&self) {
        let mut c = self.count.lock().expect("sem lock");
        while *c == 0 {
            c = self.notify.wait(c).expect("sem wait");
        }
        *c -= 1;
    }

    pub fn post(&self) {
        let mut c = self.count.lock().expect("sem lock");
        if *c < self.max {
            *c += 1;
            self.notify.notify_one();
        }
    }
}
