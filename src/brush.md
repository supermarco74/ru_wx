# `src/brush.rs` — GDI Brush (fill style)

## Purpose
Thin wrapper over a Win32 `HBRUSH` for filling rects / interiors on a `Dc`.
Mirrors wxWidgets' `wxBrush`. Tracks whether the handle is a **stock** brush
(stock brushes must **not** be `DeleteObject`d) or an owned one.

## Key types
- `BrushStyle` enum:
  - `Solid` — `CreateSolidBrush(colour)`; owned; deleted in `Drop`.
  - `Transparent` — `GetStockObject(NULL_BRUSH)`; stock; **never** deleted.
- `Brush` struct:
  - `pub colour: Colour`
  - `pub style: BrushStyle`
  - `hbrush: HBRUSH` (private)
  - `is_stock: bool` (private; set at construction time)

## Public API
```rust
impl Brush {
    pub fn new(colour: Colour, style: BrushStyle) -> Self;
    pub fn solid(colour: Colour) -> Self; // Solid(colour)
    pub fn handle(&self) -> HBRUSH;
    pub fn is_stock(&self) -> bool;       // true for the stock NULL_BRUSH
    pub fn destroy(&mut self);            // DeleteObject only if !is_stock
}
impl Drop for Brush { /* calls destroy() */ }
```

## Win32 / platform notes
- `is_stock()` **must** be checked before `DeleteObject`. Stock brushes
  (`NULL_BRUSH`, `WHITE_BRUSH`, etc.) live in a static GDI pool; calling
  `DeleteObject` on them is undefined behaviour and is exactly the bug the
  `is_stock` field is here to prevent.
- On non-Windows the struct is a stub.
- `Default for Brush` = `solid(Colour::WHITE)`.

## Tests (5)
- `brush_solid_default` — `Brush::default()` is Solid / white.
- `brush_transparent_records_style` — `new(black, Transparent)` records
  `Transparent` and `is_stock() == true`.
- `brush_styles_distinct` — Solid / Transparent round-trip with distinct
  discriminants.
- `solid_brush_handle_is_nonnull_and_owned` — `Solid` is non-null and
  `is_stock() == false`. (Windows-only.)
- `transparent_brush_handle_is_nonnull_and_stock` — `Transparent` is
  non-null and `is_stock() == true`. (Windows-only.)

## Cross-references
- `dc.rs` — used via `Dc::set_brush(&mut self, brush: Option<&Brush>)` and
  `fill_rect`'s transient brushes.
- `pen.rs` — sibling GDI object.
- `colour.rs` — `Colour` argument to `solid` / `new`.

## Quick start

```rust,no_run
use ru_wx::prelude::*;

// Solid owned brush — Drop will DeleteObject.
let bg     = Brush::solid(Colour::from_rgb(0xEE, 0xEE, 0xEE));

// Transparent stock brush — is_stock() is true, do NOT delete.
let nofill = Brush::new(Colour::BLACK, BrushStyle::Transparent);

// Use it on a paint DC inside a frame's paint event:
let brush_for_paint = bg.clone();
frame.on_paint(move |_evt| {
    let mut dc = PaintDC::new(&frame_for_paint);
    dc.set_brush(Some(&brush_for_paint));
    dc.fill_rect(0, 0, 100, 100, Colour::BLUE);
});
```
