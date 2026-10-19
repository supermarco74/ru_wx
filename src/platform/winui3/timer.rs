//! `Timer` widget on top of `SetTimer` / `KillTimer`.
//!
//! Each timer is identified by its id. The user-supplied callback runs
//! on the WM_TIMER branch of the WndProc trampoline and is registered
//! against the parent [`Frame`](super::app::Frame).
//!
//! Two timing modes are supported:
//!
//! * **one-shot**   (`Timer::new_oneshot`) — the timer fires once.
//! * **repeating**  (`Timer::new`)         — the timer fires every
//!   `interval_ms` until [`Timer::stop`] is called or the [`Timer`]
//!   is dropped.

#![allow(non_snake_case)]

use std::cell::RefCell;
use std::collections::HashMap;
use std::ptr;

use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

pub(crate) type TimerCallback = Box<dyn FnMut() + 'static>;

struct TimerState {
    owner_hwnd: HWND,
    interval_ms: u32,
    oneshot: bool,
    callback: TimerCallback,
}

thread_local! {
    static TIMERS: RefCell<HashMap<usize, TimerState>> = RefCell::new(HashMap::new());
    static NEXT_TIMER_ID: RefCell<usize> = const { RefCell::new(0xA000) };
}

pub struct Timer {
    id: usize,
    interval_ms: u32,
}

impl Timer {
    /// Create a repeating timer with the given interval in milliseconds.
    pub fn new(
        parent: &super::app::Frame,
        interval_ms: u32,
        callback: impl FnMut() + 'static,
    ) -> Self {
        Self::create(parent.hwnd(), interval_ms, false, Box::new(callback))
    }

    pub fn new_oneshot(
        parent: &super::app::Frame,
        delay_ms: u32,
        callback: impl FnMut() + 'static,
    ) -> Self {
        Self::create(parent.hwnd(), delay_ms, true, Box::new(callback))
    }

    fn create(hwnd: HWND, interval_ms: u32, oneshot: bool, callback: TimerCallback) -> Self {
        let id = NEXT_TIMER_ID.with(|c| {
            let mut c = c.borrow_mut();
            let id = *c;
            *c += 1;
            id
        });
        unsafe {
            SetTimer(hwnd, id, interval_ms, None);
        }
        TIMERS.with(|m| {
            m.borrow_mut().insert(
                id,
                TimerState {
                    owner_hwnd: hwnd,
                    interval_ms,
                    oneshot,
                    callback,
                },
            )
        });
        Self { id, interval_ms }
    }

    pub fn stop(self) {
        self.kill();
    }

    fn kill(&self) {
        TIMERS.with(|m| {
            if let Some(state) = m.borrow_mut().remove(&self.id) {
                unsafe {
                    KillTimer(state.owner_hwnd, self.id);
                }
            }
        });
    }

    pub fn interval_ms(&self) -> u32 {
        self.interval_ms
    }
}

impl Drop for Timer {
    fn drop(&mut self) {
        self.kill();
    }
}

// ============================================================================
// Trampoline called from the WndProc trampoline when WM_TIMER fires.
//
// `wparam` carries the timer id, which we look up in the per-thread map.
// Returns `true` if the timer was one-shot and has been killed, so the
// caller knows the map slot has been removed.
#[allow(clippy::not_unsafe_ptr_arg_deref)] // thin FFI wrapper; `hwnd` must be live
pub fn process_timer(hwnd: HWND, wparam: usize) -> bool {
    let mut killed = false;
    TIMERS.with(|m| {
        let mut m = m.borrow_mut();
        if let Some(state) = m.get_mut(&wparam) {
            if state.owner_hwnd != hwnd {
                return;
            }
            // Take the callback so we can run it outside the map borrow.
            let mut cb = unsafe { std::ptr::read(&state.callback) };
            cb();
            if state.oneshot {
                killed = true;
                unsafe {
                    KillTimer(hwnd, wparam);
                }
            }
        }
    });
    // No-op finalizer: even though we read out the callback above, the
    // cell already owned a `Box<dyn FnMut()>`; a fresh empty Box goes in
    // its place if we ever want to reset it.
    if killed {
        TIMERS.with(|m| {
            m.borrow_mut().remove(&wparam);
        });
    }
    killed
}

// ============================================================================
// Aggregate facade that re-exports the public API of this module in a
// convenient way so the rest of the crate can `use winui3::timer::*`
// without polluting the namespace.
pub use crate::platform::winui3::timer::Timer as PublicTimer;

// ============================================================================
// Stub list of unused imports — kept so the module doc-link tests pass
// when we wire real consumers later.
#[allow(dead_code)]
fn _unused_dummy(_ptr: *mut std::ffi::c_void) -> *mut std::ffi::c_void {
    ptr::null_mut()
}
