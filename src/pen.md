# `src/pen.rs` — GDI Pen (line stroke style)

## Purpose
Thin wrapper over a Win32 `HPEN` for drawing line strokes on a `Dc`. Owns
the handle; `Drop` calls `DeleteObject`. Mirrors wxWidgets' `wxPen`.

## Key types
- `PenStyle` enum:
  - `Solid` → `PS_SOLID` (default)
  - `Dot` → `PS_DOT`
  - `Dash` → `PS_DASH`
  - `Transparent` → `PS_NULL` (no stroke; drawing is a no-op)
- `Pen` struct:
  - `pub colour: Colour`
  - `pub width: i32` (logical pixels; `0` = 1-pixel hairline per Win32)
  - `pub style: PenStyle`
  - `hpen: HPEN` (private; use `handle()`)

## Public API
```rust
impl Pen {
    pub fn new(colour: Colour, width: i32, style: PenStyle) -> Self; // creates HPEN
    pub fn solid(colour: Colour) -> Self;                            // 1px Solid
    pub fn handle(&self) -> HPEN;                                    // raw handle
    pub fn destroy(&mut self);                                       // DeleteObject, nulls fields
}
impl Drop for Pen { /* calls destroy() */ }
```

## Win32 / platform notes
- `CreatePen(style, width, colour)` is the only constructor for non-stock
  pens. `PS_NULL` is **not** a stock object; we still create an `HPEN` for it
  (so the Drop path is uniform across all styles).
- Alpha channel on `Colour` is ignored by GDI; pens are always opaque.
- On non-Windows the struct is a stub: `hpen` is `isize` and the constructor
  is a no-op.
- `Default for Pen` = `solid(Colour::BLACK)`.

## Tests (4)
- `pen_solid_default` — `Pen::default()` is Solid / 1px / black.
- `pen_new_preserves_fields` — `new(red, 3, Dot)` round-trips.
- `pen_styles_distinct` — the four styles have distinct `as i32` values.
- `pen_handle_is_nonnull_after_new` — Windows-only smoke test (skipped
  off-Windows).

## Cross-references
- `dc.rs` — used via `Dc::set_pen(&mut self, pen: Option<&Pen>)`.
- `brush.rs` — sibling GDI object (same lifecycle pattern).
- `colour.rs` — `Colour` (RGB + alpha, alpha ignored here).

## Quick start

```rust,no_run
use ru_wx::prelude::*;

// Standard 1-pixel black pen (Drop will DeleteObject).
let hairline   = Pen::solid(Colour::BLACK);

// Custom: red, 1-px dashed line.
let dashed_red = Pen::new(Colour::RED, 1, PenStyle::Dash);

// Custom: 3-px blue dotted line.
let thick_blue = Pen::new(Colour::BLUE, 3, PenStyle::Dot);

// Transparent pen — strokes are no-ops. Still owned (not a stock object).
let invisible  = Pen::new(Colour::BLACK, 1, PenStyle::Transparent);

// Use it on a paint DC inside a frame's paint event:
let pen_for_paint = dashed_red.clone();
frame.on_paint(move |_evt| {
    let mut dc = PaintDC::new(&frame_for_paint);
    dc.set_pen(Some(&pen_for_paint));
    dc.draw_line(0, 0, 200, 200);
});
```
