# mt_scroll_bar.rs

Minitest for [`ScrollBar`](file:///f:/code/ru_wx/ru_wx/src/scroll_bar.rs) — standalone horizontal and vertical scroll bars (child `SCROLLBAR` controls, not the window-attached scroll bars used by `ScrolledWindow`).

**Run:** `cargo run --example mt_scroll_bar`

## Purpose
Demonstrate the full `ScrollBar` API surface:
- `new` (default range `0..100`, page size `10`)
- `new_full(min, max, page_size)` (custom range + page size)
- `set_range` / `get_range` round-trip
- `set_position` (thumb) / `get_position` round-trip (live value from `SBM_GETPOS`)
- `set_page_size` / `get_page_size` round-trip
- `orientation` getter
- `on_scroll` callback that pattern-matches the nine [`ScrollBarEvent`] variants

The frame hosts two scroll bars: a horizontal one at the top of the client area and a vertical one on the left side. A label in the centre describes the live state.

## Method-name collision note
> `ScrollBar` exposes an inherent `set_position(pos: i32)` that sets the **thumb** position. The `Widget` trait's `set_position(x: i32, y: i32)` (for window x/y placement) is shadowed by the inherent method, so the layout calls below go through the explicit `Widget::set_position` / `Widget::set_size` qualified syntax on the `as_widget_ref()` handle.

## Top-level flow
1. Frame 800×500.
2. **Section 1** — horizontal `ScrollBar::new(&frame, Horizontal)`:
   - `Widget::set_position(20, 60)`, `Widget::set_size(760, 16)` via `as_widget_ref().borrow_mut()`
   - `orientation()` → `Horizontal`
   - `get_range()` → `(0, 100)`, `get_page_size()` → `10`, `get_position()` → `0`
3. **Section 2** — vertical `ScrollBar::new_full(&frame, Vertical, -50, 50, 5)`:
   - positioned at `(20, 100)`, sized `16 × 360`
   - `get_range()` → `(-50, 50)`, `get_page_size()` → `5`, `get_position()` → `-50` (clamped to min)
4. **Section 3** — range round-trip:
   - `hbar.set_range(0, 1000)` → `(0, 1000)`
   - `hbar.set_position(500)` → `500`; `set_position(9999)` → `1000` (clamped)
   - `vbar.set_range(-100, 100)` → `(-100, 100)`; `vbar.set_position(0)` → `0`; `set_position(-200)` → `-100` (clamped)
5. **Section 4** — page-size round-trip:
   - `hbar.set_page_size(50)` / `(25)`; `vbar.set_page_size(10)`
6. **Section 5** — explanatory `StaticText` describing the bar configuration.
7. **Section 6** — `hbar.on_scroll(&frame, |ev| match ev { … })` with a closure matching all nine variants.

## Key APIs exercised
- [`ScrollBar::new(&frame, ScrollBarOrientation)`](file:///f:/code/ru_wx/ru_wx/src/scroll_bar.rs)
- `ScrollBar::new_full(&frame, ScrollBarOrientation, min: i32, max: i32, page_size: u32)`
- `set_range(min, max)`, `get_range() -> (i32, i32)`
- `set_position(thumb: i32)`, `get_position() -> i32`
- `set_page_size(u32)`, `get_page_size() -> u32`
- `orientation() -> ScrollBarOrientation`
- `on_scroll(&frame, FnMut(ScrollBarEvent))`
- `ScrollBarEvent::{LineUp, LineDown, PageUp, PageDown, ThumbRelease{position}, ThumbTrack{position}, Top, Bottom, EndScroll}`
- `ScrollBarOrientation::{Horizontal, Vertical}`
- `Widget::set_position` / `Widget::set_size` (qualified call to disambiguate from inherent `set_position`)

## Patterns worth noting
- **The "thumb position" semantics of `set_position`** — the integer is the live thumb value, NOT the x/y window coords.
- **Clamping on out-of-range writes** — `set_position(9999)` for a `0..1000` range returns `1000` from `get_position()`.
- **Default range + page size** — `ScrollBar::new` produces a `0..100` range with page size `10`, useful for quick demos; `new_full` is for the cases that need explicit limits.
- **`on_scroll` is registered on a specific bar** (here the horizontal one); the vertical bar has no callback attached.

## Win32 notes
- Native Win32 `SCROLLBAR` control (`SBS_HORZ` / `SBS_VERT`).
- `set_range` issues `SBM_SETRANGE` + `SBM_SETPAGESIZE`; `set_position` issues `SBM_SETPOS` + redraw; `get_position` issues `SBM_GETPOS`.
- `WM_HSCROLL` / `WM_VSCROLL` carry `SB_*` codes (LINEUP, LINEDOWN, PAGEUP, PAGEDOWN, THUMBPOSITION, THUMBTRACK, TOP, BOTTOM, ENDSCROLL); ru_wx maps them to the `ScrollBarEvent` variants.

## Cross-references
- [`scroll_bar.md`](file:///f:/code/ru_wx/ru_wx/src/scroll_bar.md)
- [`scrolled_window.md`](file:///f:/code/ru_wx/ru_wx/src/scrolled_window.md) — uses `SCROLLBAR` internally; the inverse pattern
- [`static_text.md`](file:///f:/code/ru_wx/ru_wx/src/static_text.md)
- [`widget.md`](file:///f:/code/ru_wx/ru_wx/src/widget.md) — `Widget::set_position` / `set_size`
