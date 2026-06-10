# `src/icon.rs` — SVG rasterizer + HBITMAP↔HICON

## Purpose
Two-way converter between SVGs and Win32 `HBITMAP` / `HICON`. Uses
`resvg` + `usvg` + `tiny_skia` to render SVG bytes to RGBA pixels, then
wraps them in a 32-bit DIB section. For `HICON`s, builds the mandatory
1-bpp AND-mask on top of the colour DIB and hands the pair to
`CreateIconIndirect`.

## Key functions
- `render_svg_to_pixels(svg: &[u8], w: u32, h: u32) -> Option<(Vec<u8>, u32, u32)>` —
  parses the SVG (`usvg::Tree::from_data`), renders into a
  `tiny_skia::Pixmap` of the requested size (centred if the viewBox is
  smaller), and returns the RGBA buffer + final size. Used by the
  bundle / art-provider code paths.
- `load_svg_as_hbitmap(path: impl AsRef<Path>, w: u32, h: u32) -> Option<HBITMAP>` —
  reads a file from disk and rasterizes to an `HBITMAP`.
- `svg_bytes_to_hbitmap(svg: &[u8], w: u32, h: u32) -> Option<HBITMAP>` —
  the in-memory version. Builds a 32-bit top-down DIB (matches the
  RGBA layout from `render_svg_to_pixels`) and **swizzles** RGBA → BGRA
  on the way in. The swizzle is the same as in `image.rs`; both code
  paths feed GDI's BGRA DIB format.
- `hbitmap_to_hicon(hbitmap: HBITMAP) -> HICON` — `GetObjectW` for the
  dimensions, `CreateBitmap` for a 1-bpp zero mask (zeros = every pixel
  opaque, no per-pixel alpha needed for our flat SVGs), then
  `CreateIconIndirect` on a `HICONINFO { fIcon: TRUE, hbmMask, hbmColor, .. }`.
- `svg_bytes_to_hicon(svg: &[u8], size: u32) -> Option<HICON>` —
  convenience: rasterize, then convert; deletes the intermediate `HBITMAP`
  on both success and failure paths.
- `destroy_hicon(hicon: HICON)` — null-check + `DestroyIcon`. Always
  call this on an `HICON` produced by `svg_bytes_to_hicon` /
  `hbitmap_to_hicon`; unlike `HBITMAP`, `HICON` is **not** auto-destroyed
  by `BitmapBundle::Drop`.

## Public types
- `HBitmap` — non-Windows stub struct. On non-Windows the SVG functions
  return `None` and `HICON` / `HBITMAP` types are `isize` aliases.

## Win32 / platform notes
- The 1-bpp AND-mask is **all zeros**, meaning "every pixel is opaque".
  This is the right choice for our use case (we always produce flat RGBA
  SVGs). For pre-multiplied-alpha icons, the mask would need to be the
  same size as the colour DIB and encoded per-pixel.
- `CreateDIBSection` is used (not `CreateBitmap` + `SetDIBits`) so the
  pixel buffer is directly writeable from Rust without an intermediate
  copy.
- `Centering math`: the SVG is drawn into the centre of the target
  pixmap (`offset_x = (target_w - svg_w * scale) / 2`, similarly for y);
  the area outside the centred SVG is left as fully transparent.
- `render_svg_to_pixels` is the only `pub` rendering helper; the rest
  are thin wrappers and a GDI handle factory.

## Tests
- No `#[cfg(test)] mod tests` in this file. Visual correctness is
  smoke-tested by the `icon_tray_demo` and `showcase_all` examples,
  which render every ArtId and check the resulting bitmaps are
  non-empty and the right size.

## Quick start

```rust,no_run
use ru_wx::prelude::*;
use ru_wx::icon;

// 1. Render an SVG to an HBITMAP at a specific size.
let svg: &[u8] = b"<svg viewBox=\"0 0 24 24\">\
                    <circle cx=\"12\" cy=\"12\" r=\"10\" fill=\"red\"/>\
                  </svg>";
if let Some(hbmp) = icon::svg_bytes_to_hbitmap(svg, 32, 32) {
    // 2. Hand the HBITMAP to a Bundle / ImageList / your control.
    let mut list = ImageList::new(32, 32);
    list.add_bitmap(hbmp);

    // 3. Or build an HICON for a window class / tray.
    let hicon = icon::hbitmap_to_hicon(hbmp);
    // ... set the window class icon via SetClassLongPtrW(hwnd, GCLP_HICON, hicon)
    //     or feed to IconTray::set_icon ...

    // 4. Always release HICON yourself; unlike HBITMAP, HICON is not
    //    auto-destroyed by BitmapBundle::Drop.
    icon::destroy_hicon(hicon);
}

// 5. Convenience: SVG bytes → HICON in one call.
if let Some(hicon) = icon::svg_bytes_to_hicon(svg, 16) {
    // ... use hicon ...
    icon::destroy_hicon(hicon);
}

// 6. From a file on disk:
if let Some(hbmp) = icon::load_svg_as_hbitmap("icon.svg", 48, 48) {
    // ... use hbmp; Drop or explicit DeleteObject ...
}
```

The 1-bpp AND-mask is all zeros, so every pixel is opaque. This is the right choice for flat RGBA SVGs; for pre-multiplied-alpha icons, the mask would need to be the same size as the colour DIB and encoded per-pixel.

## Cross-references
- `bitmap_bundle.rs` — `from_svg_bytes` calls `svg_bytes_to_hbitmap`
  for each requested size.
- `art_provider.rs` — built-in SVG library, then rasterized via the
  helpers here.
- `icon_tray.rs` — uses `svg_bytes_to_hicon` for the tray icon.
- `image.rs` — same RGBA→BGRA swizzle pattern.

## Example
```rust,no_run
use ru_wx::prelude::*;
use ru_wx::icon;

let svg: &[u8] = b"<svg viewBox=\"0 0 24 24\"><circle cx=\"12\" cy=\"12\" r=\"10\" fill=\"red\"/></svg>";
if let Some(hbmp) = icon::svg_bytes_to_hbitmap(svg, 32, 32) {
    let hicon = icon::hbitmap_to_hicon(hbmp);
    // ... show in a tray or window ...
    // The intermediate HBITMAP is owned by the HICON's ICONINFO; it is
    // released by DestroyIcon on the hicon below.
    icon::destroy_hicon(hicon);
}
```
