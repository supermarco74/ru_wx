# static_bitmap.rs

Read-only image display control (`wxStaticBitmap` analog). Shows a single bitmap or icon inside a parent window. The control does not respond to clicks — for interactive images use a `Button` with a bitmap.

## Purpose
Display a `BitmapBundle` (or raw `HBITMAP`/`HICON`) inside a static region of the layout. The control can either size itself to the image's natural dimensions or honour an explicit size from the caller.

## Key Types
- `StaticBitmap` — public struct.
- `StaticBitmapInner` (private) — Win32 `HWND` for the `STATIC` child plus an `ImageKind` discriminator.
- `ImageKind` — `Bitmap` (HBITMAP) or `Icon` (HICON). The owned handle is released in `Drop`.
- `RawBitmap` — borrowed handle passed to `set_bitmap`; the control does **not** take ownership.

## Key Methods
- `StaticBitmap::new<W: Window>(parent: &W, bundle: &BitmapBundle, size: Option<Size>) -> Self` — General-purpose constructor. `size == None` means the control uses the bundle's intrinsic dimensions.
- `StaticBitmap::with_bitmap<W: Window>(parent: &W, hbitmap: HBITMAP, w: u32, h: u32) -> Self` — Windows-only. Wrap an externally-owned `HBITMAP`. The control does not delete it on drop.
- `StaticBitmap::with_icon<W: Window>(parent: &W, hicon: HICON, size: Option<Size>) -> Self` — Windows-only. Wrap an externally-owned `HICON`.
- `set_bitmap(&self, bmp: RawBitmap)` — Replaces the current image (any prior owned handle is released).
- `set_raw_bitmap(&self, hbitmap: HBITMAP)` — Sets an externally-owned bitmap; the control does not take ownership.
- `set_icon(&self, hicon: HICON)` — Sets an externally-owned icon.
- `clear(&self)` — Removes the image and releases the previously-owned handle.

## Win32 Notes
- Window class: built-in `STATIC`.
- Styles: `SS_BITMAP` (`0x000E`) for bitmaps, `SS_ICON` (`0x0003`) for icons, combined with `WS_CHILD | WS_VISIBLE`. `SS_CENTERIMAGE` (`0x0200`) centres the image in the control rect; `SS_REALSIZECONTROL` (`0x0800`) sizes the control to the image.
- Messages: `STM_SETIMAGE` (`0x0172`) assigns, `STM_GETIMAGE` (`0x0173`) queries. `IMAGE_BITMAP` (`0`) and `IMAGE_ICON` (`1`) are the WPARAMs.
- `release_current` calls `DeleteObject` for owned bitmaps and `DestroyIcon` for owned icons. Externally-owned handles (set via `set_raw_bitmap`/`set_icon`) are not destroyed.
- `clone_bitmap` uses `GetObjectW` + `CreateDIBSection` + `GetDIBits` to copy a bitmap so the control can own its image independently. `clone_icon` uses `CopyIcon`.
- `Drop` impl runs `release_current` so a `StaticBitmap` value cleans up after itself.

## Tests
- `default_dimensions_are_positive` — Newly-constructed control has non-zero size when given a positive-size bundle.
- `image_kind_variants_compare_distinctly` — `ImageKind::Bitmap != ImageKind::Icon`.

## Quick start

```rust
use ru_wx::prelude::*;

// frame is the owning Frame.
let bundle = BitmapBundle::from_svg_file("assets/icons/star.svg", 64)
    .expect("svg present");

// Sizing: None = use the bitmap's natural size.
let pic = StaticBitmap::new(&frame, &bundle, None);

// Or pass an explicit (w, h) in pixels:
let pic_fixed = StaticBitmap::new(&frame, &bundle, Some(Size { w: 128, h: 128 }));

// Replace the image at runtime (the previous one is released).
pic.set_bitmap(RawBitmap { hbitmap: other_hbmp, w: 64, h: 64 });
```

The control is non-interactive — for clickable images use a
[`BitmapButton`](./bitmap_button.md). The `Drop` impl releases any owned
image handle.

## See Also
- [`bitmap_bundle.rs`](./bitmap_bundle.md) — high-level bundle abstraction
- [`bitmap.rs`](../img) — raw bitmap creation and I/O
- [`icon.rs`](./icon.md) — icon construction
- [`static_text.rs`](./static_text.md) — sibling static control
