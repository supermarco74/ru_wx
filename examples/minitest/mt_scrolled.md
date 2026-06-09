# mt_scrolled.rs

Minitest for [`ScrolledWindow`](file:///f:/code/ru_wx/ru_wx/src/scrolled_window.rs) — scrollable container with virtual size.

**Run:** `cargo run --example mt_scrolled`

## Purpose
Demonstrate the full `ScrolledWindow` API surface:
- `new` (default 200×200, virtual size `(0, 0)`)
- `set_virtual_size` / `get_virtual_size` round-trip
- `set_view_position` / `get_view_position` round-trip
- `on_scroll` callback registration that pattern-matches the nine [`ScrolledWindowScrollEvent`] variants

The contents are a single long `StaticText`; the scroll bars let the user pan over its full extent.

## Top-level flow
1. Frame 700×500.
2. `ScrolledWindow::new(&frame)`; resize to 660×440 via `Widget::set_size`.
3. **Virtual-size round-trip**:
   - `get_virtual_size()` → `(0, 0)` (default)
   - `set_virtual_size(4*660, 4*440)` → both bars appear
   - `set_virtual_size(0, 0)` → bars disappear
   - `set_virtual_size(2000, 1200)` → bars reappear
4. **View-position round-trip**:
   - `get_view_position()` → `(0, 0)`
   - `set_view_position(50, 30)` → `(50, 30)`
   - `set_view_position(0, 0)` → `(0, 0)` again
5. Add a long `StaticText` as the scrollable content.
6. Register `on_scroll` with a closure that pattern-matches all nine [`ScrolledWindowScrollEvent`] variants.

## Key APIs exercised
- [`ScrolledWindow::new(&frame)`](file:///f:/code/ru_wx/ru_wx/src/scrolled_window.rs)
- `set_virtual_size(w, h)`, `get_virtual_size() -> (i32, i32)`
- `set_view_position(x, y)`, `get_view_position() -> (i32, i32)`
- `on_scroll(FnMut(ScrolledWindowScrollEvent))`
- `ScrolledWindowScrollEvent::{LineUp, LineDown, PageUp, PageDown, ThumbRelease{position}, ThumbTrack{position}, Top, Bottom, EndScroll}`
- `Widget::set_size(w, h)` (reached via `as_widget_ref().borrow_mut()`)

## Patterns worth noting
- **Virtual size drives the scroll bars** — when the virtual size is `(0, 0)` (the default) the window has no scroll bars; making it larger than the visible size makes the OS show them.
- **`ThumbTrack` vs `ThumbRelease`** — `ThumbTrack` fires repeatedly while the user drags the thumb; `ThumbRelease` fires once on mouse-up. The closure distinguishes them by pattern.
- **Content reparenting** — the `StaticText` is created with `&scrolled` as the parent, so it lives inside the scrolled window's HWND.

## Win32 notes
- `ScrolledWindow` is built on a `WC_STATIC` (or a custom `WndClass`) that re-forwards `WM_HSCROLL` / `WM_VSCROLL` to ru_wx's `on_scroll` callback.
- `set_virtual_size` updates the internal `scrollbar` range (`SIF_RANGE`) and the page size (`SIF_PAGE`) so the thumb and arrows work as expected.
- `set_view_position` issues `SetScrollPos` + `ScrollWindowEx` to update the visible area.

## Cross-references
- [`scrolled_window.md`](file:///f:/code/ru_wx/ru_wx/src/scrolled_window.md)
- [`static_text.md`](file:///f:/code/ru_wx/ru_wx/src/static_text.md)
- [`widget.md`](file:///f:/code/ru_wx/ru_wx/src/widget.md) — `Widget::set_size`
- [`scroll_bar.md`](file:///f:/code/ru_wx/ru_wx/src/scroll_bar.md) — sibling widget (`SCROLLBAR` child control)
