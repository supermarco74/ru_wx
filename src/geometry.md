# geometry.rs

Tiny geometry primitives shared across the widget layer.

## Purpose

Two types: `Rect` (position + size) and `Colour` (RGBA with a Win32 `COLORREF` converter). Lives at the bottom of the dependency stack so every widget can build layouts without dragging GDI in.

## Key types

- **`Rect { x: i32, y: i32, width: u32, height: u32 }`** — `#[derive(Default)]` so `Rect::default()` is `(0, 0, 0, 0)`. Methods:
  - `Rect::new(x, y, w, h)`.
  - `contains(px, py) -> bool` — half-open: lower-left corner inclusive, upper-right corner exclusive.
- **`Colour { r, g, b, a: u8 }`** — RGBA. Constants: `WHITE`, `BLACK`, `LIGHT_GREY`. Default is `WHITE`. Methods:
  - `Colour::new(r, g, b, a)` (const).
  - `to_colorref(self) -> u32` (Windows-only) — packs to `0x00BBGGRR` as the Win32 `COLORREF` API expects.

## Win32 notes

- `to_colorref` is `#[cfg(target_os = "windows")]` and is the only function in the module with a platform guard.
- The `b | g | r` packing matches the historical Windows convention (least significant byte is **red**, not blue); the alpha byte is intentionally dropped because `COLORREF` is 24 bits.

## Tests

The module's `#[cfg(test)] mod tests` block locks in:

- `Rect::new` preserves all four fields.
- `Rect::default()` is the zero origin.
- `Rect::contains` is half-open (lower-left inclusive, upper-right exclusive).
- `Colour::WHITE` / `BLACK` / `LIGHT_GREY` constant values.
- `Colour::default()` is `WHITE`.
- `to_colorref` produces the expected `0x00BBGGRR` for pure red / green / blue / mid-grey.

## See also

- [`widget.rs`](./widget.md) — `Widget::rect` returns `Rect`.
- [`dpi.rs`](./dpi.md) — coordinate scaling (logical ↔ physical pixels).
- [`brush.rs`](./brush.md), [`pen.rs`](./pen.md) — both consume `Colour` via `to_colorref`.

## Quick start

```rust
use ru_wx::prelude::*;

// Rect: position + size.
let r = Rect::new(10, 20, 100, 50);
assert!(r.contains(15, 25));      // lower-left inclusive
assert!(!r.contains(200, 25));    // outside
let _ = Rect::default();          // (0, 0, 0, 0)

// Colour: RGBA, with helpers for the common cases.
let red   = Colour::new(255, 0,   0,   255);
let grey  = Colour::LIGHT_GREY;   // (192, 192, 192, 255)

#[cfg(target_os = "windows")]
let colorref: u32 = red.to_colorref();  // 0x000000FF (0x00BBGGRR)

// Use the colour with a brush when drawing on a Dc.
let _brush = Brush::new(grey);
```

`to_colorref()` is Windows-only; on other targets the function is not
exposed at all. The alpha byte is dropped because Win32 `COLORREF` is
24 bits.
