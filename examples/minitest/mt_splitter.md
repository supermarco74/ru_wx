# mt_splitter.rs

Minitest for [`SplitterWindow`](file:///f:/code/ru_wx/ru_wx/src/splitter_window.rs) — resizable two-pane container.

**Run:** `cargo run --example mt_splitter`

## Purpose
Demonstrate the full splitter API surface:
- Default `new` + `split_vertically` (left | right)
- `split_horizontally` (top / bottom)
- `set_orientation` / `orientation` round-trip
- `set_sash_position` / `get_sash_position` round-trip + clamping
- `on_sash_drag` callback that pattern-matches the three [`SashEvent`] variants

The two pane `HWND`s are [`Panel`](file:///f:/code/ru_wx/ru_wx/src/panel.rs) instances — `Panel` is the only built-in widget other than `Frame` that implements the [`Window`] trait, and the splitter API takes raw `HWND`s for its panes. Each panel hosts a single `StaticText` label so it is obvious which pane is which after the drag.

## Top-level flow
1. Frame 800×500.
2. **Section 1** — vertical splitter (top half):
   - `SplitterWindow::new(&frame)`, sized 760×220
   - `left_pane = Panel::new(&frame)`, `right_pane = Panel::new(&frame)`
   - Add `StaticText` labels to each pane
   - `vertical_split.split_vertically(left_pane.hwnd(), right_pane.hwnd())` (Windows-gated)
3. **Section 2** — horizontal splitter (bottom half, y=230, 760×230):
   - top_pane / bottom_pane via `Panel::new`
   - `horizontal_split.split_horizontally(top_pane.hwnd(), bottom_pane.hwnd())`
4. **Orientation round-trip** — flip the bottom splitter `Horizontal` → `Vertical` → `Horizontal` and read `orientation()` after each.
5. **Sash position round-trip** — `get_sash_position()` → `100` (default mid-point); `set_sash_position(150)`; `set_sash_position(200)` (clamped to `dim - SASH_GRAB`).
6. **Sash drag callback** — `vertical_split.on_sash_drag(|ev| match ev { SashEvent::DragStart, DragMove{position}, DragEnd{position} })` — the closure body never fires from this synchronous test, it just needs to compile and accept every variant.

## Key APIs exercised
- [`SplitterWindow::new(&frame)`](file:///f:/code/ru_wx/ru_wx/src/splitter_window.rs)
- `SplitterWindow::split_vertically(hwnd_left, hwnd_right)`
- `SplitterWindow::split_horizontally(hwnd_top, hwnd_bottom)`
- `SplitterWindow::set_orientation(SplitterOrientation)`
- `SplitterWindow::orientation() -> SplitterOrientation`
- `SplitterWindow::set_sash_position(i32)`
- `SplitterWindow::get_sash_position() -> i32`
- `SplitterWindow::on_sash_drag(FnMut(SashEvent))`
- `SashEvent::{DragStart, DragMove{position}, DragEnd{position}}`
- `SplitterOrientation::{Vertical, Horizontal}`
- `Panel::new(&frame)`, `Panel::hwnd() -> HWND`
- `StaticText::new(&panel, "…")`
- `Widget::set_position` / `Widget::set_size` (reached via `as_widget_ref().borrow_mut()`)

## Patterns worth noting
- **Panes must be `Window` implementors** — the splitter API takes raw `HWND`s because the OS needs a parent for each pane. `Panel` is the lightweight container of choice (no extra UI overhead, no style).
- **`#[cfg(target_os = "windows")]` gates the live calls** — the closures and the `split_*` calls would otherwise link against Win32-only types. The orientation / sash-position getters don't need the gate because they're pure read-only reads.
- **Sash position clamping** — the OS reserves a small `SASH_GRAB` strip at the edge; setting the position past `dim - SASH_GRAB` silently clamps to that limit.

## Win32 notes
- `SplitterWindow` is built on a custom `WndClass` that re-forwards `WM_LBUTTONDOWN` / `WM_MOUSEMOVE` / `WM_LBUTTONUP` from the sash region to ru_wx's `on_sash_drag` callback.
- The sash itself is a thin `STATIC` child of the splitter, painted with `SS_ETCHEDFRAME` + 4-pixel grip dots.
- `split_vertically` / `split_horizontally` call `SetParent` to re-parent the two pane HWNDs under the splitter and position them on either side of the sash.
- `set_orientation` is a wrapper that internally calls the right `split_*` based on the new orientation, preserving the sash's previous position in pixels.

## Cross-references
- [`splitter_window.md`](file:///f:/code/ru_wx/ru_wx/src/splitter_window.md)
- [`panel.md`](file:///f:/code/ru_wx/ru_wx/src/panel.md) — the only built-in `Window` pane
- [`static_text.md`](file:///f:/code/ru_wx/ru_wx/src/static_text.md)
- [`widget.md`](file:///f:/code/ru_wx/ru_wx/src/widget.md) — `Widget::set_position` / `set_size`
- [`window.md`](file:///f:/code/ru_wx/ru_wx/src/window.md) — `Window` trait, the trait bound on `split_*`
