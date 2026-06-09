# `src/bitmap_bundle.rs` — Multi-resolution bitmap for HiDPI

## Purpose
`BitmapBundle` is a list of `HBITMAP`s, each at a different resolution, plus
the **logical size** they all represent. Used for toolbars, menus, list
controls, and any other surface that should pick the closest match to the
current DPI. Mirrors wxWidgets' `wxBitmapBundle`.

## Key types
- `pub struct RawBitmap` (#[derive(Clone, Copy)]):
  - `hbitmap: HBITMAP`
  - `width: u32`
  - `height: u32`
- `BitmapBundle`:
  - `bitmaps: Vec<RawBitmap>`
  - `logical_size: (u32, u32)` — the design size (e.g. 16×16 for a menu
    icon); populated from the first `add` / `from_raw_bitmap` / etc.

## Public API
```rust
impl RawBitmap { /* pub fields, ctor not usually called directly */ }

impl BitmapBundle {
    pub fn new() -> Self;                                                 // empty
    pub fn from_raw_bitmap(rb: RawBitmap) -> Self;
    pub fn from_bitmap(bmp: &Bitmap) -> Self;
    pub fn add(&mut self, bmp: &Bitmap) -> &mut Self;                     // first add sets logical_size
    pub fn from_svg_bytes(svg: &[u8], sizes: &[(u32, u32)]) -> Self;      // renders at each size
    pub fn from_svg_path(path: impl AsRef<Path>, sizes: &[(u32, u32)]) -> Option<Self>;
    pub fn best_for_size(&self, target: (u32, u32)) -> Option<RawBitmap>; // see algorithm
    pub fn best_for_dpi(&self, dpi: u32) -> Option<RawBitmap>;            // target = logical_size * dpi/96
    pub fn best_for_hwnd(&self, hwnd: HWND) -> Option<RawBitmap>;         // GetDC + GetDeviceCaps(LOGPIXELSX)
    pub fn handle(&self) -> HBITMAP;                                      // first entry, or 0
    pub fn logical_size(&self) -> (u32, u32);
}
impl Drop for BitmapBundle { /* DeleteObject on every HBITMAP */ }
```

## HiDPI algorithms
- `best_for_size(target)` — pick the entry that minimizes
  `|w - target.0| + |h - target.1|`. Exact matches win, then the next
  closest. Empty bundle returns `None`.
- `best_for_dpi(dpi)` — `scale = dpi / 96` (integer division); `target =
  (logical_size.0 * scale, logical_size.1 * scale)`; defers to
  `best_for_size`. At 96 DPI the target is the logical size itself; at
  192 DPI it is doubled; at 144 DPI it is 1.5× (which is why the SVG
  bundle in `art_provider.rs` is rendered at `[base, 1.5×, 2×]`).
- `best_for_hwnd(hwnd)` — `GetDC(hwnd)` → `GetDeviceCaps(LOGPIXELSX)` →
  `ReleaseDC` → defers to `best_for_dpi`. The implementation calls
  `crate::platform::win32::get_device_caps_dpi(hwnd)` for the Win32
  bit.

## Win32 / platform notes
- `Drop` walks `bitmaps` and calls `DeleteObject` on every non-zero
  `HBITMAP`. Don't share a `RawBitmap` with a `Bitmap` — that would
  double-free.
- `from_svg_bytes` calls `crate::icon::svg_bytes_to_hbitmap` for each
  size, so SVG support is automatic.
- On non-Windows the struct exists with empty fields; `handle()` returns 0.

## Tests
- No `#[cfg(test)] mod tests` in this file. Multi-resolution selection
  is exercised by the `aui_toolbar_demo` / `icon_tray_demo` examples,
  which check the right size is picked at 100% / 150% / 200% DPI.

## Quick start

```rust,no_run
use ru_wx::prelude::*;

// 1. Build a multi-resolution bundle by hand.
let mut b = BitmapBundle::new();
b.add(&Bitmap::new(16, 16));   // 100% / 96  DPI
b.add(&Bitmap::new(24, 24));   // 150% / 144 DPI
b.add(&Bitmap::new(32, 32));   // 200% / 192 DPI
// The first add() sets the logical_size to (16, 16).
assert_eq!(b.logical_size(), (16, 16));

// 2. Or build one from raw HBITMAP(s):
let raw = RawBitmap { hbitmap: /* … */, width: 16, height: 16 };
let b2 = BitmapBundle::from_raw_bitmap(raw);

// 3. Pick the best fit for a target pixel size / DPI / HWND.
let on_dpi_200 = b.best_for_dpi(192);   // picks the 32×32 entry
let on_window  = b.best_for_hwnd(frame.hwnd());
let on_pixels  = b.best_for_size((40, 40));

// 4. The typical real-world use: render an SVG once per size at startup,
//    then have the bundle pick the right one for the current DPI.
let svg: &[u8] = include_bytes!("../assets/icons/star.svg");
let icons = BitmapBundle::from_svg_bytes(svg, &[(16, 16), (24, 24), (32, 32)]);
let bundle = icons.unwrap();
let h = bundle.best_for_dpi(frame.dpi().value()).unwrap();
```

`Drop` walks `bitmaps` and calls `DeleteObject` on every non-zero `HBITMAP`, so the bundle owns its GDI resources for life.

## Cross-references
- `bitmap.rs` — the single-resolution building block.
- `art_provider.rs` — the canonical producer; bundles are built at
  `[base, base*1.5, base*2]`.
- `icon.rs` — SVG → HBITMAP rendering for the bundle entries.
- `dpi.rs` / `platform::win32` — `best_for_hwnd` and `best_for_dpi`.

## Example
```rust,no_run
use ru_wx::prelude::*;

let mut b = BitmapBundle::new();
b.add(&Bitmap::new(16, 16));
b.add(&Bitmap::new(24, 24));
b.add(&Bitmap::new(32, 32));
// On a 200% DPI monitor, `b.best_for_dpi(192)` picks the 32×32.
```
