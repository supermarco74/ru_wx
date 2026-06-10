# gauge.rs

Progress bar backed by the Win32 `msctls_progress32` common control.

## Purpose
- Implements `Gauge` (mirrors `wxGauge`): a horizontal or vertical bar that fills from 0 to a maximum value as the caller pushes new values.
- Three constructors cover the common shapes; `new_with_style` exposes the full style + marquee knob.
- Supports **indeterminate (marquee)** mode where the bar continuously scrolls and `set_value` is a no-op.

## Key Types
- `GaugeStyle` — enum: `Horizontal`, `SmoothHorizontal` (PBS_SMOOTH), `Vertical` (PBS_VERTICAL), `SmoothVertical` (PBS_VERTICAL | PBS_SMOOTH).
- `GaugeInner` — `hwnd`, `id`, `rect`, `range`, `value`, `indeterminate`, `enabled`, `visible`. Single allocation behind `Rc<RefCell<…>>`.
- `Gauge` — public handle. Cheap to clone (`Rc`).

## Key Functions/Methods
- `Gauge::new<W: Window>(parent, range)` — horizontal segmented bar.
- `Gauge::new_smooth<W: Window>(parent, range)` — horizontal smooth (no segments).
- `Gauge::new_vertical<W: Window>(parent, range)` — vertical segmented.
- `Gauge::new_with_style<W: Window>(parent, range, style, indeterminate)` — full constructor; the last flag enables `PBS_MARQUEE` and starts the marquee animation via `pulse`.
- `set_range(&self, range)` — `PBM_SETRANGE32` (32-bit safe); clamps cached value to new range.
- `get_range(&self) -> i32` — cached, no Win32 call.
- `set_value(&self, value)` — clamps to `[0, range]`, ignores calls in marquee mode, then `PBM_SETPOS`.
- `get_value(&self) -> i32` — `PBM_GETPOS` (also refreshes cache).
- `increment(&self, delta) -> i32` — clamps + `PBM_DELTAPOS`, returns the new value.
- `set_step(step) / step()` — `PBM_SETSTEP` + `PBM_STEPIT` (one-shot progress).
- `pulse() / stop_pulse()` — `PBM_SETMARQUEE` with wparam=1/0 and lparam=30 ms (~33 fps).
- `set_bar_colour(colour: Colour)` — `PBM_SETBARCOLOR` with `colour.to_colorref()`. Only effective on Windows visuals that respect it; the segmented Vista style ignores it.
- `id()`, `as_widget_ref`, plus the standard `Widget` impl.

## Win32 Notes
- Class: `msctls_progress32` (requires `InitCommonControlsEx` with `ICC_PROGRESS_CLASS`, done in `app.rs`).
- Style flags: `PBS_SMOOTH = 0x01` (no segment borders), `PBS_VERTICAL = 0x04`, `PBS_MARQUEE = 0x08`. Created 200×20 (horizontal) or 200×200 (vertical, but the constructor's default is 20 height — caller typically uses `set_size` to elongate).
- Messages used: PBM_SETPOS (0x0402), PBM_DELTAPOS (0x0403), PBM_SETSTEP (0x0404), PBM_STEPIT (0x0405), PBM_SETRANGE32 (0x0406), PBM_GETPOS (0x0408), PBM_SETBARCOLOR (0x0409), PBM_SETMARQUEE (0x040A). `PBM_SETRANGE` (0x0401, 16-bit) is also declared but `#[allow(dead_code)]` — the 32-bit variant is used everywhere.
- Marquee mode: `PBS_MARQUEE` must be set at creation time (Win32 limitation) — the constructor toggles it via the `indeterminate` parameter and immediately calls `pulse` to start the animation.
- `pulse` is also callable on a determinate bar; the OS treats it as a no-op there.
- The cached `value` is updated by `get_value` so a subsequent `get_value` after a long break returns the OS's authoritative position, not a stale local one.

## Quick start

```rust
use ru_wx::prelude::*;

// frame is the owning Frame.
let g = Gauge::new(&frame, 100);   // 0..=100

// Drive a long task:
for i in 0..=100 {
    g.set_value(i);
    // do_work_for_step(i);
}

// Or increment by a delta (returns the new value).
let new = g.increment(5);

// Indeterminate (marquee) bar — set_value becomes a no-op.
let m = Gauge::new_with_style(&frame, 0, GaugeStyle::Horizontal, true);
m.pulse();
```

`PBS_MARQUEE` can only be set at creation time; the `indeterminate` arg
to `new_with_style` toggles it. `set_bar_colour` only affects Windows
visuals that respect it — the segmented Vista style ignores it.

## See Also
- [`slider.rs`](slider.md) — sibling "common control" widget (trackbar).
- [`spin_ctrl.rs`](spin_ctrl.md) — composite of EDIT + up-down for integer stepping.
- [`widget.rs`](../core/widget.md) — `Widget` trait implementation.
- [`geometry.rs`](../core/geometry.md) — `Colour` type used in `set_bar_colour`.
- [`lib.rs`](../lib.md) — `next_control_id()` allocator.
