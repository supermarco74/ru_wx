# `src/bitmap.rs` — Single-resolution GDI bitmap

## Purpose
`Bitmap` is a single-resolution GDI bitmap (one `HBITMAP`, one `width × height`).
Contrast with [`BitmapBundle`](bitmap_bundle.md), which holds multiple
resolutions for HiDPI. Mirrors wxWidgets' `wxBitmap`.

## Key types
- `Bitmap` struct:
  - `pub width: u32`
  - `pub height: u32`
  - `empty: bool` — `true` means the handle is a Windows "null bitmap" (a
    Win32 sentinel that is not 0 but is "non-functional"); used by
    `is_null()`. Set to `true` on Windows when the constructor can't
    allocate, and unconditionally on non-Windows.
  - `handle: isize` — raw `HBITMAP`. Stored as `isize` (not `HBITMAP`) so
    `Bitmap` is `Clone + Send + 'static` without an unsafe `impl Send`
    for the raw handle.

## Public API
```rust
impl Bitmap {
    pub fn new(width: u32, height: u32) -> Self;          // CreateDIBSection
    pub fn from_hbitmap(hbmp: HBITMAP, w: u32, h: u32) -> Self; // unsafe ownership transfer
    pub fn handle(&self) -> HBITMAP;                       // cast from isize
    pub fn is_null(&self) -> bool;                         // empty (Windows) or true (non-Windows)
    pub fn destroy(&mut self);                             // DeleteObject, sets empty/handle=0
}
impl Clone for Bitmap { /* allocates a NEW HBITMAP via CreateDIBSection */ }
impl Drop for Bitmap { /* calls destroy() */ }
```

## Win32 / platform notes
- `new` allocates a 32-bit DIB section (`BITMAPINFOHEADER` with
  `biBitCount = 32`, `biHeight = height` — bottom-up is the Win32
  default; `Image::to_bitmap` uses `biHeight = -height` for top-down).
- `from_hbitmap` is the **unsafe ownership transfer** path. The caller
  hands over the raw handle and the `Bitmap` takes responsibility for
  `DeleteObject` in `Drop`. If you wrap a bitmap you did not create, you
  will double-free.
- `Clone` does a deep copy: it builds a new `HBITMAP` and copies the
  bits row-by-row. This is intentional so two `Bitmap` values do not
  share a single GDI handle.
- On non-Windows the struct is a stub: `new` is a no-op, `is_null`
  returns `true`, and the handle is always 0.

## Tests (4)
- `new_blank_records_dimensions` — `new(32, 16)` → `width == 32`, `height == 16`.
- `new_blank_has_nonnull_handle` — Windows: handle is non-zero after `new`.
- `destroy_nullifies_handle` — after `destroy()`, `is_null() == true`,
  `handle() == 0`.
- `from_hbitmap_records_dimensions_and_is_non_null` — ownership transfer
  preserves dimensions and produces a non-null handle on Windows.

## Cross-references
- `image.rs` — `Image::to_bitmap()` is the common producer of a `Bitmap`.
- `bitmap_bundle.rs` — multi-resolution variant for HiDPI.
- `icon.rs` — `svg_bytes_to_hbitmap` is an alternative producer.
- `dc.rs` — `Dc::draw_bitmap(&Bitmap, x, y)` and `MemoryDC::select_bitmap(&Bitmap)`.

## Quick start

```rust,no_run
use ru_wx::prelude::*;

// Create a 32x32 32-bit DIB section (HBITMAP).
let bmp = Bitmap::new(32, 32);
assert!(!bmp.is_null());

// Wrap an externally-created HBITMAP (ownership transfer — Bitmap will
// DeleteObject in Drop; do NOT DeleteObject the original separately).
// SAFETY: caller guarantees `hbmp` is a valid HBITMAP and unique ownership
//         is being handed to this Bitmap.
let raw_hbmp: HBITMAP = /* acquired from elsewhere */ todo!();
let wrapped = unsafe { Bitmap::from_hbitmap(raw_hbmp, 64, 64) };

// Clone = a deep copy (separate HBITMAP, separate pixels).
let copy = bmp.clone();

// Use it on a paint DC inside a frame's paint event:
let bmp_for_paint = bmp.clone();
frame.on_paint(move |_evt| {
    let mut dc = PaintDC::new(&frame_for_paint);
    dc.draw_bitmap(&bmp_for_paint, 10, 10);
});

// Drop deletes the HBITMAP; `is_null()` becomes true.
drop(bmp);
```
