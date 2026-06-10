# static_line.rs

Horizontal or vertical line separator (`wxStaticLine` analog). A thin etched divider used to break up sections of a layout.

## Purpose
Visual separator with zero interaction. Lays out as a 200×2 (horizontal) or 2×200 (vertical) etched line. Pair with a sizer to control its actual size and orientation in the layout.

## Key Types
- `StaticLineOrientation` — `Horizontal` (default) or `Vertical`.
- `StaticLine` — public struct.

## Key Methods
- `StaticLine::new_horizontal<W: Window>(parent: &W) -> Self` — 200×2 horizontal line.
- `StaticLine::new_vertical<W: Window>(parent: &W) -> Self` — 2×200 vertical line.
- `StaticLine::new<W: Window>(parent: &W, orientation: StaticLineOrientation) -> Self` — explicit orientation.
- `orientation(&self) -> StaticLineOrientation` — getter.

## Win32 Notes
- Window class: built-in `STATIC`.
- Styles: `SS_ETCHEDHORZ` (`0x0010`) for horizontal, `SS_ETCHEDVERT` (`0x0011`) for vertical.
- Always child (`WS_CHILD | WS_VISIBLE`).
- Etched style renders as a faint 3D groove. The line is drawn by the system; no custom paint handling is required.

## Tests
- `default_orientation_is_horizontal` — Default is `Horizontal`.
- `orientations_compare_distinctly` — `Horizontal != Vertical` and the enum derives `PartialEq`.

## Quick start

```rust
use ru_wx::prelude::*;

let hr = StaticLine::new_horizontal(&frame);   // 200×2 etched line
let vr = StaticLine::new_vertical(&frame);     // 2×200 etched line

// Add to a sizer like any other widget:
let mut s = BoxSizer::vertical();
s.add(label.as_widget_ref());
s.add(hr.as_widget_ref());
s.add(button.as_widget_ref());
```

The visible width of a horizontal line is governed by the sizer — give it a non-zero `proportion` (or a stretchable neighbour) to make it span the available width.

## See Also
- [`static_text.rs`](static_text.md) — sibling static control for textual labels
- [`static_box.rs`](static_box.md) — labelled frame, can contain a `StaticLine` inside its border
- [`sizer.rs`](../containers/sizer.md) — required for layout; `StaticLine` size in a `BoxSizer` is typically governed by `expand` and proportion
