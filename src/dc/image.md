# `src/image.rs` — Format-independent image (RGBA8 pixel buffer)

## Purpose
`Image` is a format-agnostic in-memory pixel buffer. It can be loaded from
common image formats (via the `image` crate), edited pixel-by-pixel, and
flushed to a GDI `HBITMAP` for display. The buffer is always 8-bit-per-channel
RGBA (premultiplied straight), row-major, top-to-bottom, no padding.

## Key types
- `pub type Rgba = (u8, u8, u8, u8);` — `(R, G, B, A)`.
- `ImageError` enum:
  - `IoError(std::io::Error)` — `From` impl enables `?` on file reads.
  - `DecodeError(String)` — image-format-specific decode failure.
  - `InvalidSize` — width/height is zero or `usize`-overflow on `w*h*4`.
  - `Display + std::error::Error` are implemented.
- `Image` struct:
  - `pub width: u32`
  - `pub height: u32`
  - `pub pixels: Vec<u8>` — length is `width * height * 4`, RGBA8 row-major.

## Public API
```rust
impl Image {
    pub fn new(width: u32, height: u32) -> Result<Self, ImageError>;     // zero-filled
    pub fn load_from_memory(data: &[u8]) -> Result<Self, ImageError>;    // uses image crate
    pub fn load_from_file(path: impl AsRef<Path>) -> Result<Self, ImageError>;
    pub fn get_pixel(&self, x: u32, y: u32) -> Option<Rgba>;
    pub fn set_pixel(&mut self, x: u32, y: u32, pixel: Rgba) -> bool;    // true on success
    pub fn pixels(&self) -> &[u8];
    pub fn pixels_mut(&mut self) -> &mut [u8];
    pub fn is_null(&self) -> bool;                                       // zero-dim or empty
    pub fn to_bitmap(&self) -> Bitmap;                                   // 32-bit top-down DIB
}
```

## Win32 / platform notes
- `to_bitmap()` creates a 32-bit DIB with `biHeight = -height` (top-down,
  matches our buffer's row-major top-to-bottom layout). Pixel bytes are
  **swizzled** from RGBA → BGRA as they are written to the DIB: the source
  `(R, G, B, A)` becomes `(B, G, R, A)` in the destination. This is the
  only place in the codebase where the swizzle happens; once the DIB is
  populated, every other consumer reads BGRA correctly.
- `set_pixel` / `get_pixel` use the same row-major layout; both are
  bounds-checked and return `None` / `false` for out-of-range coordinates.
- `Image` does **not** own any GDI resources — `to_bitmap` returns a
  `Bitmap` (which does), and the `Image` can be safely dropped afterwards.

## Tests (5)
- `new_records_dimensions_and_zeros` — `new(2, 3)` → dimensions match,
  `pixels` is 24 zero bytes.
- `get_set_pixel_round_trips` — write `Rgba(1, 2, 3, 4)` to `(0, 0)`, read
  it back.
- `is_null_for_zero_size` — `new(0, 0)` is an error (or returns an
  `is_null` image depending on impl); non-zero is not null.
- `load_from_memory_rejects_garbage` — `&[0xFF, 0x00, 0x00]` returns
  `Err(DecodeError(_))`.
- `to_bitmap_returns_correct_dimensions` — 32-bit DIB is `(w, h)` and
  pixels round-trip through the swizzle.

## Quick start

```rust,no_run
use ru_wx::prelude::*;

// 1. Create a zero-filled RGBA8 buffer.
let mut img = Image::new(64, 64).unwrap();

// 2. Set / get individual pixels.
img.set_pixel(0, 0, (255, 0, 0, 255));        // red, top-left
let px = img.get_pixel(0, 0);                  // Some((255, 0, 0, 255))

// 3. Bulk access for fast pixel loops:
for chunk in img.pixels_mut().chunks_exact_mut(4) {
    chunk[0] = 0;   // R
    chunk[1] = 0;   // G
    chunk[2] = 0;   // B
    chunk[3] = 0;   // A
}

// 4. Load a PNG / JPG / etc. from bytes:
let png_bytes: &[u8] = /* …read from disk or HTTP… */;
let loaded = Image::load_from_memory(png_bytes)?;
// or from a path:
let from_file = Image::load_from_file("logo.png")?;

// 5. Hand it to a DC (Image::to_bitmap builds a 32-bit top-down DIB):
let bmp = img.to_bitmap();
frame.on_paint(move |hwnd| {
    let mut dc = unsafe { PaintDC::new(hwnd) };
    dc.draw_bitmap(&bmp, 10, 10);
});
```

`Image` does not own any GDI resources — `to_bitmap` returns a `Bitmap` (which does), and the `Image` can be safely dropped afterwards.

## Cross-references
- `bitmap.rs` — output of `Image::to_bitmap`.
- `bitmap_bundle.rs` — multi-resolution image bundles (icons etc.).
- `icon.rs` — SVG path (uses image crate indirectly via the `tiny_skia`
  pipeline, not via `image`).
- `art_provider.rs` — produces SVGs that are eventually rasterized into
  `Image`-equivalent bitmaps.

## Example
```rust,no_run
use ru_wx::prelude::*;

let img = Image::new(64, 64).unwrap();
let mut img = img;
img.set_pixel(0, 0, (255, 0, 0, 255)); // red, top-left
let bmp = img.to_bitmap();
```
