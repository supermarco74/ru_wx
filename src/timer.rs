//! Repeating / one-shot timer.
//!
//! On Windows, the timer is realised with `SetTimer` on the parent
//! frame's `HWND`. When the interval elapses, Win32 delivers a
//! `WM_TIMER` message to the window. Because the frame's WndProc only
//! knows how to dispatch `WM_COMMAND` / `WM_NOTIFY` / user-defined
//! `WM_APP + n` messages, we route the timer ticks through a dedicated
//! user-defined message in the `WM_APP + 0x100 + n` range (the tray
//! already uses the `WM_APP + n` range for its shell messages; the
//! gap avoids any chance of collision).
//!
//! Use [`Timer::start`] to start ticking, [`Timer::stop`] to cancel.
//! The `on_tick` closure is invoked once per tick while the timer is
//! running.

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;

use crate::frame::Frame;

#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::WindowsAndMessaging::{KillTimer, SetTimer, WM_APP};

/// User-defined message range base (WM_APP is 0x8000). We use
/// `WM_APP + 0x100 + n` so the timer's tick id is well above `WM_APP`
/// (the tray uses the `WM_APP + n` range for its shell messages; the
/// gap avoids any chance of collision).
const TIMER_MSG_BASE: u32 = WM_APP + 0x100;

/// Monotonically increasing per-process `WM_TIMER` redirect message id.
static NEXT_TIMER_MSG: AtomicU32 = AtomicU32::new(1);

#[cfg(target_os = "windows")]
struct TimerState {
    /// Win32 `SetTimer` id (also the `wparam` value of the `WM_TIMER`
    /// message we receive).
    timer_id: u32,
    /// `WM_APP + 0x100 + n` message id; dispatched by the frame's
    /// WndProc to the registered handler.
    msg_id: u32,
    /// User's on_tick callback. `None` until [`Timer::on_tick`] is
    /// called.
    on_tick: Option<Box<dyn FnMut()>>,
    /// `true` between `start()` and `stop()`.
    running: bool,
    /// Most-recently-requested interval in milliseconds (used by
    /// `start_again`).
    last_interval_ms: u32,
    /// `true` if the timer is configured to fire only once. The first
    /// tick auto-stops the timer (with no `KillTimer` leak) so the
    /// `on_tick` callback never fires a second time.
    one_shot: bool,
}

#[cfg(target_os = "windows")]
impl TimerState {
    fn new(timer_id: u32, msg_id: u32) -> Self {
        Self {
            timer_id,
            msg_id,
            on_tick: None,
            running: false,
            last_interval_ms: 1000,
            one_shot: false,
        }
    }
}

/// A `wxTimer`-like repeating / one-shot timer.
pub struct Timer {
    #[cfg(target_os = "windows")]
    frame: Frame,
    #[cfg(target_os = "windows")]
    state: Rc<RefCell<TimerState>>,
    /// The frame's HWND captured at construction (so we don't need to
    /// re-borrow the frame for `KillTimer` / `SetTimer` on drop or
    /// `stop`).
    #[cfg(target_os = "windows")]
    hwnd: windows_sys::Win32::Foundation::HWND,
}

#[cfg(target_os = "windows")]
impl Timer {
    /// Create a new timer attached to the given frame.
    ///
    /// The timer is *not running* after this call. Use [`Timer::start`]
    /// to begin ticking.
    pub fn new(frame: &Frame) -> Self {
        use windows_sys::Win32::Foundation::HWND;

        let hwnd = frame.hwnd();
        let timer_id = NEXT_TIMER_MSG.fetch_add(1, Ordering::Relaxed);
        let msg_id = TIMER_MSG_BASE + NEXT_TIMER_MSG.fetch_add(1, Ordering::Relaxed);
        let state = Rc::new(RefCell::new(TimerState::new(timer_id, msg_id)));

        // Register a WM_APP-based message handler that fires the
        // user's on_tick closure. We use the same take / call / put
        // pattern as `Frame::command_handlers` to avoid re-entrant
        // RefCell borrows.
        let state_clone = state.clone();
        let hwnd_for_handler = frame.hwnd();
        frame.register_tray_message_handler(
            msg_id,
            Box::new(move |_lparam| {
                // Pull what we need out of state, then drop the borrow
                // before invoking the callback (so the callback can
                // re-enter Timer methods without dead-locking the
                // RefCell).
                let cb = {
                    let mut s = state_clone.borrow_mut();
                    let cb = s.on_tick.take();
                    if s.one_shot && s.running {
                        // One-shot: stop the Win32 timer now so the
                        // kernel doesn't queue a second tick behind
                        // this handler. We don't reset `last_interval_ms`
                        // so `start_again` still works.
                        let id = s.timer_id;
                        s.running = false;
                        // SAFETY: `hwnd_for_handler` is the frame's
                        // HWND captured at construction; `id` is the
                        // `SetTimer` id we registered.
                        unsafe {
                            windows_sys::Win32::UI::WindowsAndMessaging::KillTimer(
                                hwnd_for_handler as _,
                                id as usize,
                            );
                        }
                    }
                    cb
                };
                if let Some(mut c) = cb {
                    c();
                    // Re-arm the callback so the user can call
                    // `start` / `start_one_shot` again later. (We
                    // took it above to avoid re-entrancy; restore
                    // it now that the callback has returned.)
                    state_clone.borrow_mut().on_tick = Some(c);
                }
            }),
        );

        Timer {
            frame: frame.clone(),
            state,
            hwnd: hwnd as HWND,
        }
    }

    /// Set the callback fired on every tick.
    pub fn on_tick<F: FnMut() + 'static>(&self, callback: F) {
        self.state.borrow_mut().on_tick = Some(Box::new(callback));
    }

    /// Start the timer with the given interval. Replaces any
    /// previously-running timer.
    pub fn start(&self, interval: Duration) {
        let ms = interval.as_millis().min(u32::MAX as u128) as u32;
        if ms == 0 {
            // Win32 treats 0 as "no timer"; clamp to 1ms-equivalent.
            // (We pick 1ms as the minimum; the user just passed a 0
            // interval by accident.)
            return;
        }
        self.state.borrow_mut().last_interval_ms = ms;

        let mut state = self.state.borrow_mut();
        // Reset one-shot: a regular `start` is always repeating.
        state.one_shot = false;
        if state.running {
            // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
            unsafe {
                KillTimer(self.hwnd, state.timer_id as usize);
            }
        }
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            SetTimer(self.hwnd, state.timer_id as usize, ms, None);
        }
        state.running = true;
    }

    /// Start the timer to fire **once** after `interval`, then auto-stop.
    ///
    /// The first tick invokes the registered `on_tick` callback once and
    /// then kills the underlying Win32 timer. After firing, the timer
    /// can be re-armed with another `start_one_shot` (or a regular
    /// `start` / `start_again`).
    pub fn start_one_shot(&self, interval: Duration) {
        let ms = interval.as_millis().min(u32::MAX as u128) as u32;
        if ms == 0 {
            return;
        }
        self.state.borrow_mut().last_interval_ms = ms;

        let mut state = self.state.borrow_mut();
        state.one_shot = true;
        if state.running {
            // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
            unsafe {
                KillTimer(self.hwnd, state.timer_id as usize);
            }
        }
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            SetTimer(self.hwnd, state.timer_id as usize, ms, None);
        }
        state.running = true;
    }

    /// `true` if the timer is currently configured as a one-shot timer
    /// (the next call to `start` will reset this to `false`).
    pub fn is_one_shot(&self) -> bool {
        self.state.borrow().one_shot
    }

    /// Set or clear the one-shot flag. When called while the timer is
    /// running, the change takes effect on the **next** tick (the
    /// current running mode is not changed mid-flight). To switch
    /// modes immediately, stop and re-start.
    pub fn set_one_shot(&self, one_shot: bool) {
        self.state.borrow_mut().one_shot = one_shot;
    }

    /// The most recently requested interval. Returns `None` if the
    /// timer has never been started (in which case the Win32 default
    /// of 1000 ms has not been overridden).
    pub fn interval(&self) -> Option<Duration> {
        let s = self.state.borrow();
        if s.last_interval_ms == 0 {
            None
        } else {
            Some(Duration::from_millis(s.last_interval_ms as u64))
        }
    }

    /// Stop the timer if running. Idempotent.
    pub fn stop(&self) {
        let mut state = self.state.borrow_mut();
        if !state.running {
            return;
        }
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            KillTimer(self.hwnd, state.timer_id as usize);
        }
        state.running = false;
    }

    /// Restart the timer using the most-recently-requested interval.
    pub fn start_again(&self) {
        let ms = self.state.borrow().last_interval_ms;
        self.start(Duration::from_millis(ms as u64));
    }

    /// `true` if the timer is currently running.
    pub fn is_running(&self) -> bool {
        self.state.borrow().running
    }
}

#[cfg(target_os = "windows")]
impl Drop for Timer {
    fn drop(&mut self) {
        let (running, timer_id, msg_id) = {
            let state = self.state.borrow();
            (state.running, state.timer_id, state.msg_id)
        };
        if running {
            // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
            unsafe {
                KillTimer(self.hwnd, timer_id as usize);
            }
        }
        // Detach the message handler from the frame.
        self.frame.unregister_tray_message_handler(msg_id);
    }
}

// ---- Non-Windows stubs ----

#[cfg(not(target_os = "windows"))]
pub struct Timer;

#[cfg(not(target_os = "windows"))]
impl Timer {
    pub fn new(_frame: &Frame) -> Self {
        Self
    }
    pub fn on_tick<F: FnMut() + 'static>(&self, _callback: F) {}
    pub fn start(&self, _interval: Duration) {}
    pub fn stop(&self) {}
    pub fn start_again(&self) {}
    pub fn start_one_shot(&self, _interval: Duration) {}
    pub fn is_one_shot(&self) -> bool {
        false
    }
    pub fn set_one_shot(&self, _one_shot: bool) {}
    pub fn interval(&self) -> Option<Duration> {
        None
    }
    pub fn is_running(&self) -> bool {
        false
    }
}
