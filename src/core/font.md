# `src/font.rs` — GDI Font (face, size, style, DPI)

## Purpose
Builder-style `FontDesc` + owning `Font` over a Win32 `HFONT` created via
`LOGFONTW` + `CreateFontW`. Mirrors wxWidgets' `wxFont` / `wxFontInfo`.

## Key types
- `FontDesc` (builder):
  - `pub face_name: String` (e.g. `"Segoe UI"`)
  - `pub point_size: i32` (logical points)
  - `pub bold: bool`, `pub italic: bool`, `pub underline: bool`
  - `pub dpi: u32` (defaults to `96`; override for HiDPI rendering)
  - `FontDesc::new(face, size)` — start a builder.
  - `.bold()`, `.italic()`, `.underline()`, `.with_dpi(dpi)` — chainable.
  - `Default` = `Segoe UI` 9pt, no decoration, dpi=96.
- `Font`:
  - `hfont: HFONT` (private; use `hfont()`)
  - `desc: FontDesc` (private; use `desc()`)
  - `Clone` is **deep**: each clone allocates a fresh `HFONT` so two `Font`
    values can be `Drop`-ed independently without a double-free.

## Public API
```rust
impl FontDesc {
    pub fn new(face: impl Into<String>, point_size: i32) -> Self;
    pub fn bold(mut self) -> Self;
    pub fn italic(mut self) -> Self;
    pub fn underline(mut self) -> Self;
    pub fn with_dpi(mut self, dpi: u32) -> Self;
}

impl Font {
    pub fn new(desc: FontDesc) -> Self;          // builds the LOGFONTW
    pub fn default_system() -> Self;             // Segoe UI 9pt @ 96 DPI
    pub fn desc(&self) -> &FontDesc;
    pub fn hfont(&self) -> HFONT;                // Windows
}
```

## Win32 / platform notes
- Pixel height is computed as `-(point_size * dpi / 72)`. The negation is the
  Win32 convention: positive `lfHeight` = line height (cell), negative =
  **character** height (what "9pt" actually means to a user). The `* dpi / 72`
  factor converts points → pixels at the supplied DPI (default 96).
- `LOGFONTW` fields used:
  - `lfHeight` (computed as above)
  - `lfWeight` = `FW_BOLD` or `FW_NORMAL`
  - `lfItalic`, `lfUnderline` (0/1)
  - `lfCharSet = DEFAULT_CHARSET`
  - `lfOutPrecision = OUT_DEFAULT_PRECIS`
  - `lfClipPrecision = CLIP_DEFAULT_PRECIS`
  - `lfQuality = CLEARTYPE_QUALITY`
  - `lfPitchAndFamily = DEFAULT_PITCH | FF_DONTCARE`
- `face_name` is `wcsncpy`-clipped to `LF_FACESIZE - 1` chars; longer names
  are silently truncated.
- `CreateFontW` is called in the private `create_hfont` helper.
- `Clone` allocates a new `HFONT` because `Drop` would otherwise `DeleteObject`
  a shared handle.

## Tests
- The module does not currently declare its own `#[cfg(test)] mod tests`.
  All public types are exercised indirectly by widget tests in
  `frame.rs` / `panel.rs`.

## Quick start

```rust,no_run
use ru_wx::prelude::*;

// 1. Default system font (Segoe UI 9pt @ 96 DPI).
let sys = Font::default_system();

// 2. Build a custom font with a chainable builder.
let big_bold = Font::new(
    FontDesc::new("Segoe UI", 14)
        .bold()
        .italic()
);

// 3. HiDPI-aware: ask for 12pt at the current monitor's DPI.
let dpi = frame.dpi().value();   // e.g. 192 on a 200% monitor
let scaled = Font::new(
    FontDesc::new("Segoe UI", 12)
        .bold()
        .with_dpi(dpi)
);

// 4. Clone is a deep copy (each clone allocates its own HFONT), so a
//    captured font in a closure is independent of the original:
let font_for_paint = big_bold.clone();
frame.on_paint(move |hwnd| {
    let mut dc = unsafe { PaintDC::new(hwnd) };
    // Fonts are not yet auto-selected by the Dc trait; the typical
    // pattern is to select the HFONT into the HDC at the call site:
    // let prev = unsafe { SelectObject(dc.handle(), font_for_paint.hfont()) };
    // ... draw text ...
    // unsafe { SelectObject(dc.handle(), prev) };
    let (w, h) = dc.text_extent("hello");
    println!("text extent: {w} x {h}");
});
```

`Clone` allocates a new `HFONT` because `Drop` would otherwise `DeleteObject` a shared handle.

## Cross-references
- `dc.rs` — `Dc` does **not** have a `set_font` method. Fonts are applied to
  the DC at the call site by the caller (e.g. via raw `SelectObject(hdc,
  font.hfont())` if/when needed) — see `frame.rs` paint handlers.
- `dpi.rs` — the `dpi` value to pass to `FontDesc::with_dpi`.

## Example
```rust,no_run
use ru_wx::prelude::*;

let f = Font::new(
    FontDesc::new("Segoe UI", 12)
        .bold()
        .with_dpi(192)  // 200% HiDPI
);
let _hfont = f.hfont();
```
