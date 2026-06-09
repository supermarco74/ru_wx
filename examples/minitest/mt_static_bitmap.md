# mt_static_bitmap.rs

Minitest for [`StaticBitmap`](file:///f:/code/ru_wx/ru_wx/src/static_bitmap.rs) — image display in three flavours.

**Run:** `cargo run --example mt_static_bitmap`

## Purpose
Demonstrate the four public constructors of `StaticBitmap` and the `set_bitmap` / `clear` lifecycle:
1. An **empty** `StaticBitmap` (just the control, no image)
2. A `StaticBitmap` bound to a [`BitmapBundle`](file:///f:/code/ru_wx/ru_wx/src/bitmap_bundle.rs) built from a multi-resolution SVG (best-fit is selected at request time)
3. A `StaticBitmap` with a procedurally created [`Bitmap`](file:///f:/code/ru_wx/ru_wx/src/bitmap.rs) (a solid colour DIB)
4. A `StaticBitmap` with an `HICON` produced from inline SVG bytes
5. The `set_bitmap` / `clear` lifecycle methods

## Embedded assets
| Const | Source | Purpose |
|---|---|---|
| `STAR_SVG` | `assets/icons/star.svg` | bootstrap-icons star — used to build the bundle |
| `info_svg` | inline `br#"…"` literal | info-circle glyph — used to build the HICON |

## Top-level flow
1. Frame 460×360.
2. Header `StaticText` "Image display — empty / bundle / bitmap / icon".
3. **(1)** `StaticBitmap::with_size(&frame, 32, 32)` — empty placeholder, label "1. empty".
4. **(2)** `BitmapBundle::from_svg_bytes(STAR_SVG, &[(16,16),(24,24),(32,32)])` + `StaticBitmap::new(&frame, &bundle, (32, 32))` — `best_for_size` is invoked internally to pick the closest match. Label "2. bundle (SVG → 32×32)".
5. **(3)** `let red_bmp = Bitmap::new(32, 32)` + `StaticBitmap::with_bitmap(&frame, red_bmp.handle(), 32, 32)`. On Windows this is a 32-bit DIB section. Label "3. raw 32×32 HBITMAP".
6. **(4)** `let hicon = svg_bytes_to_hicon(info_svg, 32).unwrap_or(std::ptr::null_mut());` + `StaticBitmap::with_icon(&frame, hicon, (32, 32))`. Label "4. icon (inline SVG → HICON)".
7. **(5)** Lifecycle:
   - `let green_bmp = Bitmap::new(24, 24);`
   - `let lifecycle = StaticBitmap::with_size(&frame, 24, 24);`
   - `lifecycle.set_bitmap(RawBitmap { hbitmap: green_bmp.handle(), width: 24, height: 24 });`
   - `lifecycle.clear();` — strips the bitmap back to empty.
   - Label "5. set_bitmap then clear()".
8. Stack all 10 widgets (5 bitmaps + 5 labels + header) in a vertical sizer.
9. `let _keep = (red_bmp, green_bmp, bundle);` — keep the bitmaps alive for the lifetime of the message loop; `Bitmap::drop` releases the underlying `HBITMAP`, so we cannot let them go out of scope.
10. `app.run(frame)`.

## Key APIs exercised
- [`StaticBitmap::with_size(&frame, w, h)`](file:///f:/code/ru_wx/ru_wx/src/static_bitmap.rs) — empty placeholder
- `StaticBitmap::new(&frame, &BitmapBundle, (w, h))` — bundle-driven
- `StaticBitmap::with_bitmap(&frame, hbitmap: isize, w, h)` — raw `HBITMAP`
- `StaticBitmap::with_icon(&frame, hicon, (w, h))` — raw `HICON`
- `StaticBitmap::set_bitmap(RawBitmap { hbitmap, width, height })`
- `StaticBitmap::clear()`
- [`BitmapBundle::from_svg_bytes(&[u8], &[(w, h); N])`](file:///f:/code/ru_wx/ru_wx/src/bitmap_bundle.rs) — multi-resolution SVG → bundle
- [`Bitmap::new(w, h)`](file:///f:/code/ru_wx/ru_wx/src/bitmap.rs), `Bitmap::handle() -> isize`
- [`svg_bytes_to_hicon(&[u8], size) -> Option<isize>`](file:///f:/code/ru_wx/ru_wx/src/icon.rs) (re-export of the icon module's HICON builder)
- `RawBitmap { hbitmap, width, height }` — plain struct passed to `set_bitmap`

## Patterns worth noting
- **`RawBitmap` is a value type** — it carries the handle, width and height, so `StaticBitmap` doesn't have to query the OS for the size every frame.
- **`_keep` is required** — the two `Bitmap` values and the `BitmapBundle` must outlive the message loop or their `Drop` impls will `DeleteObject` the Win32 handles, leaving the `StaticBitmap` with a dangling HBITMAP.
- **`with_icon` accepts a possibly-null HICON** — `unwrap_or(std::ptr::null_mut())` is the standard way to feed a result that might fail; the control renders as empty if the HICON is `NULL`.
- **`clear` is symmetric with `set_bitmap`** — both touch the same internal HWND state.

## Win32 notes
- `StaticBitmap` is a native `STATIC` control with `SS_BITMAP` / `SS_ICON` styles set at construction time.
- `set_bitmap` sends `STM_SETIMAGE` with `IMAGE_BITMAP`; `with_icon` uses `IMAGE_CURSOR` / `IMAGE_ICON` depending on the handle.
- `Bitmap` uses `CreateDIBSection` with `BI_RGB`, top-down layout, 32-bit per pixel — the most efficient shape for blitting via `BitBlt` / `StretchBlt`.
- `svg_bytes_to_hicon` builds the HICON via the resvg → tiny_skia → usvg pipeline, then `ICONINFO` + `CreateIconIndirect` to wrap the resulting HBITMAPs (colour + mask) into a single HICON.

## Cross-references
- [`static_bitmap.md`](file:///f:/code/ru_wx/ru_wx/src/static_bitmap.md)
- [`bitmap.md`](file:///f:/code/ru_wx/ru_wx/src/bitmap.md)
- [`bitmap_bundle.md`](file:///f:/code/ru_wx/ru_wx/src/bitmap_bundle.md) — HiDPI multi-resolution
- [`icon.md`](file:///f:/code/ru_wx/ru_wx/src/icon.rs) — `svg_bytes_to_hicon`
- [`static_text.md`](file:///f:/code/ru_wx/ru_wx/src/static_text.md) — sibling widget, identical control class
- [`assets/icons/star.svg`](file:///f:/code/ru_wx/ru_wx/assets/icons/star.svg)
