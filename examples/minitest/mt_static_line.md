# mt_static_line.rs

Minitest for [`StaticLine`](file:///f:/code/ru_wx/ru_wx/src/static_line.rs) — horizontal and vertical separators.

**Run:** `cargo run --example mt_static_line`

## Purpose
Demonstrate the two [`StaticLineOrientation`] variants placed between two pieces of text. On Windows the line is drawn as an `SS_ETCHEDHORZ` / `SS_ETCHEDVERT` `STATIC` control, so the orientation property is also exposed at runtime via `orientation()`.

## Top-level flow
1. Frame 420×260.
2. **(1)** Verify all three construction forms:
   - `let line_h_explicit = StaticLine::new(&frame, StaticLineOrientation::Horizontal);` → `assert_eq!(line_h_explicit.orientation(), Horizontal);`
   - `let line_h_sugar = StaticLine::new_horizontal(&frame);` → `assert_eq!(line_h_sugar.orientation(), Horizontal);`
   - `let line_v = StaticLine::new_vertical(&frame);` → `assert_eq!(line_v.orientation(), Vertical);`
3. **(2)** Pack the demo widgets:
   - `header = StaticText::new(&frame, "Above the horizontal line");`
   - `sub    = StaticText::new(&frame, "Below the horizontal line");`
   - `left   = StaticText::new(&frame, "Left");`
   - `right  = StaticText::new(&frame, "Right");`
4. Stack in a vertical sizer: header / horizontal-line / sub / left / vertical-line / right.
5. `app.run(frame)`.

## Key APIs exercised
- [`StaticLine::new(&frame, StaticLineOrientation)`](file:///f:/code/ru_wx/ru_wx/src/static_line.rs)
- `StaticLine::new_horizontal(&frame)` — sugar for `new(frame, Horizontal)`
- `StaticLine::new_vertical(&frame)` — sugar for `new(frame, Vertical)`
- `StaticLine::orientation() -> StaticLineOrientation`
- `StaticLineOrientation::{Horizontal, Vertical}`

## Patterns worth noting
- **`new_horizontal` / `new_vertical` are pure sugar** — they exist so the call site reads naturally without a `use` of the orientation enum. The implementation is literally `Self::new(parent, Orientation::Horizontal)`.
- **`assert_eq!` on `orientation()`** — locks in the value at construction time, before the user can interact with the line. This is the canonical "API surface test" pattern in minitests.
- **Static lines are non-interactive** — there is no `on_click` / `on_paint` callback; the OS draws the etched line and that's the whole widget.

## Win32 notes
- `StaticLine` is a native `STATIC` control with `SS_ETCHEDHORZ` (default) or `SS_ETCHEDVERT` style.
- The default size is the OS default (200×2 for horizontal, 2×200 for vertical); sizers compute the actual size from the parent.
- The OS paints the etched line in `WM_PAINT`; ru_wx does not intervene.

## Cross-references
- [`static_line.md`](file:///f:/code/ru_wx/ru_wx/src/static_line.md)
- [`static_text.md`](file:///f:/code/ru_wx/ru_wx/src/static_text.md) — sibling widget, identical control class
- [`sizer.md`](file:///f:/code/ru_wx/ru_wx/src/sizer.md) — `BoxSizer::vertical`
