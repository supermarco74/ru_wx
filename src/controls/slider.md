# slider.rs

Continuous-value slider (trackbar) backed by the Win32 `msctls_trackbar32` common control.

## Purpose
- Implements `Slider` (mirrors `wxSlider`): a movable thumb on a `[min, max]` integer scale.
- Supports both orientations: `Slider::new` (horizontal) and `Slider::new_vertical`.
- Reports changes through a `WM_COMMAND` handler on the parent `Frame` (the trackbar synthesises `WM_HSCROLL` / `WM_VSCROLL` as `WM_COMMAND` notifications).

## Key Types
- `SliderInner` — `hwnd`, `id`, `rect`, `min`, `max`, `value` (cached, refreshed on `get_value`), `enabled`, `visible`. Single allocation behind `Rc<RefCell<…>>`.
- `Slider` — public handle. Cheap to clone (`Rc`).
- No separate `SliderValueChangeFn` type — `on_value_change` takes a `FnMut() + 'static` directly; the handler re-queries the value via `TBM_GETPOS` so the user does not need to read `wparam`.

## Key Functions/Methods
- `Slider::new<W: Window>(parent, min, max, initial)` — creates a horizontal trackbar, 200×30, with `TBS_AUTOTICKS | TBS_BOTH` (ticks above and below).
- `Slider::new_vertical<W: Window>(parent, min, max, initial)` — vertical, 200×200, `TBS_VERT | TBS_LEFT`.
- `set_range(&self, min, max)` — `TBM_SETRANGEMIN` + `TBM_SETRANGEMAX` (fRedraw=1) and clamps cached value to the new range.
- `set_value(&self, value)` — clamps to `[min, max]`, updates cache, then `TBM_SETPOS` (fRedraw=1).
- `get_value(&self) -> i32` — `TBM_GETPOS`, also updates the cached value.
- `get_min / get_max / get_range` — `TBM_GETRANGEMIN` / `TBM_GETRANGEMAX` (or cache on non-Windows).
- `set_tick_freq(freq)` — `TBM_SETTICFREQ` (one tick every `freq` units).
- `set_page_size(page) / set_line_size(line)` — `TBM_SETPAGESIZE` / `TBM_SETLINESIZE` for Page-Up/Page-Down and arrow keys.
- `on_value_change<F: FnMut() + 'static>(&self, frame, callback)` — registers a `WM_COMMAND` handler on the frame; the callback fires for every scroll event (thumb drag, arrow key, page key) and re-reads the position with `TBM_GETPOS`.
- `id()`, `as_widget_ref`, plus the standard `Widget` impl.

## Win32 Notes
- Class: `msctls_trackbar32` (requires `InitCommonControlsEx` with `ICC_BAR_CLASSES`, done in `app.rs`).
- Style flags used: `WS_CHILD | WS_VISIBLE | TBS_AUTOTICKS` + orientation-specific bits. `TBS_HORZ` is 0 (default), `TBS_VERT = 0x0002`, `TBS_BOTH = 0x0008` (ticks above+below), `TBS_LEFT = 0x0004` (ticks on left side of vertical).
- Message codes: TBM_GETPOS = WM_USER+21 (0x0415), TBM_SETPOS = WM_USER+5 (0x0405), TBM_GETRANGEMIN = WM_USER+1, TBM_GETRANGEMAX = WM_USER+2, TBM_SETRANGEMIN = WM_USER+3, TBM_SETRANGEMAX = WM_USER+4, TBM_SETTICFREQ = WM_USER+20, TBM_SETLINESIZE = WM_USER+23, TBM_SETPAGESIZE = WM_USER+25. The file derives them from `WM_USER = 0x0400` to keep the table self-contained and version-stable.
- Trackbar scroll events are reported as `WM_COMMAND` (not `WM_HSCROLL` / `WM_VSCROLL`); the `HIWORD(wparam)` carries the SB_* code (SB_THUMBPOSITION, SB_ENDSCROLL, …). The handler does not parse this — it just refreshes the cached value with `TBM_GETPOS` and fires the user callback.
- The initial value is clamped via `value.max(min).min(max)` before being sent to the control, so callers can pass an out-of-range initial without surprising the control.
- `set_range` sends `fRedraw=1` so the tick marks reposition immediately.

## Quick start

```rust
use ru_wx::prelude::*;

// frame is the owning Frame.
let s = Slider::new(&frame, 0, 100, 25);
s.set_tick_freq(10);

let label = StaticText::new(&frame, "Value: 25");
let label_for_cb = label.clone();
s.on_value_change(&frame, move || {
    let v = s.get_value();
    label_for_cb.set_label(&format!("Value: {v}"));
});

// Vertical variant:
let sv = Slider::new_vertical(&frame, 0, 10, 0);
```

The handler re-queries the position with `TBM_GETPOS` on every fire, so
the user does not need to read `wparam`. The handler also fires for
arrow keys and Page-Up/Page-Down, not just mouse drag.

## See Also
- [`spin_ctrl.rs`](spin_ctrl.md) — discrete-step sibling (composite of EDIT + up-down).
- [`frame.rs`](../window/frame.md) — `register_command_handler` used by `on_value_change`.
- [`widget.rs`](../core/widget.md) — `Widget` trait implementation.
- [`lib.rs`](../lib.md) — `next_control_id()` allocator.
