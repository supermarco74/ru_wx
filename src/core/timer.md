# `timer.rs` — `Timer` (Win32 SetTimer wrapper)

A wrapper around the Win32 **`SetTimer`** / **`KillTimer`** API, with
periodic and one-shot modes. Timers are owned by a `Frame` and dispatch
through a user-defined message number per timer.

## Purpose

- A `Timer` runs a callback at a fixed interval (or exactly once) without
  blocking the message pump.
- Each timer gets a **dedicated message number** in the
  `WM_APP + 0x100 + n` range, allocated from a global `AtomicU32`
  counter starting at 1. The +0x100 offset puts timers well above the
  `WM_APP + n` range used by the icon tray, so the two namespaces don't
  collide.
- The WndProc intercepts each timer's message, runs the user callback,
  and (for one-shot timers) auto-calls `KillTimer`.

## Public type

```rust
pub struct Timer { /* Rc<RefCell<TimerInner>> */ }
```

## Public API

| Method | Purpose |
|---|---|
| `new(frame) -> Self` | Create a stopped timer attached to `frame`. |
| `on_tick<F: FnMut() + 'static>(&self, F)` | Register the per-tick callback. |
| `start(&self, interval)` | Start periodic ticking at the given `Duration`. |
| `start_one_shot(&self, interval)` | Start a one-shot timer. |
| `stop(&self)` | Stop and kill the underlying Win32 timer. |
| `start_again(&self)` | Restart the timer with the previously-set interval. |
| `is_running(&self) -> bool` | Whether the timer is currently scheduled. |
| `is_one_shot(&self) -> bool` | Whether the timer is in one-shot mode. |
| `set_one_shot(&self, one_shot)` | Toggle one-shot mode (call before `start_again`). |
| `interval(&self) -> Option<Duration>` | The currently-configured interval, if any. |

`Drop` calls `KillTimer` and unregisters the per-message handler.

## Quick start

```rust,no_run
use std::time::Duration;
use ru_wx::prelude::*;

// 1. Build a periodic timer attached to a frame.
let tick_timer = Timer::new(&frame);
let mut counter = 0u32;
let counter_for_tick = tick_timer.clone();
let frame_for_tick   = frame.clone();
tick_timer.on_tick(move || {
    counter += 1;
    println!("tick #{counter}");
    if counter >= 5 {
        counter_for_tick.stop();   // stop the periodic ticking
        frame_for_tick.close_window();
    }
});
tick_timer.start(Duration::from_millis(250));

// 2. Or a one-shot timer (auto-stops after firing once).
let one_shot = Timer::new(&frame);
one_shot.on_tick(|| println!("fired once"));
one_shot.start_one_shot(Duration::from_secs(2));

// 3. Inspect / re-arm later.
if one_shot.is_running() {
    one_shot.stop();
}
one_shot.set_one_shot(true);
one_shot.start_again();   // uses the previously-set interval
```

`Drop` calls `KillTimer` and unregisters the per-message handler, so a timer that's allowed to fall out of scope is cleaned up automatically. The handler is re-entrancy-safe: a user's `on_tick` callback can freely call `start` / `stop` / `start_again` on the timer without deadlocking on the `RefCell` borrow.

## Win32 notes

- `SetTimer(hwnd, nIDEvent, ms, null)` — `nIDEvent = 0` so the timer
  posts `WM_TIMER` to the frame's WndProc; the *callback dispatch* is
  done in Rust by routing on the per-timer message number, not by the
  Win32 timer-callback pointer.
- The WndProc maps the per-timer `WM_APP + 0x100 + n` message back to
  the `Rc<RefCell<TimerInner>>` it stored at registration time, then
  invokes the user's `on_tick` callback.

## Re-entrancy-safe callback

The handler **takes** the callback out of the `RefCell`, calls it, then
**puts it back**:

```text
let cb = inner.borrow_mut().on_tick.take();
if let Some(mut c) = cb {
    c();              // user code runs without holding the RefCell
    inner.borrow_mut().on_tick = Some(c);
}
```

This is the same pattern used by `Frame`'s command/notify dispatchers
and by the `Tab` selection-change handler. It prevents a re-entrant
`start`/`stop`/etc. from the user's own callback from deadlocking on
the `RefCell` borrow.

## One-shot auto-stop

For one-shot timers, the handler additionally calls `KillTimer` on the
first tick. The `running` flag is cleared and the user can re-arm with
`start_again`.

## Non-Windows stub

`#[cfg(not(target_os = "windows"))]` provides an empty impl block with
the same method signatures but `start`, `stop`, etc. are no-ops. The
goal is to keep the API surface usable for non-Windows targets (e.g. CI
doc builds) without a real `Win32` runtime.

## Cross-references

- [`frame.md`](../window/frame.md) — the timer's parent HWND; supplies the
  message-pump and WndProc.
- [`icon_tray.md`](../chrome/icon_tray.md) — sibling of `Timer` in the message-
  number namespace; tray uses `WM_APP + n` while timers use
  `WM_APP + 0x100 + n`.
