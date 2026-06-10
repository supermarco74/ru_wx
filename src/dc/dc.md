# `src/dc.rs` — Device Contexts (GDI drawing targets)

## Purpose
The four Win32 GDI device-context flavors (`PaintDC`, `ClientDC`, `WindowDC`,
`MemoryDC`) wrapped in a `Dc` trait that gives a uniform drawing API
(lines, rects, ellipses, text, bitmaps, state: pen/brush/text colour/background).
The trait is the public contract; the four structs are thin wrappers over
the matching Win32 `BeginPaint` / `GetDC` / `CreateCompatibleDC` lifecycle.

## Key types
- `Dc` trait — the public drawing surface. All four DC types implement it.
  - State: `set_pen`, `set_brush`, `set_text_color`, `set_bk_color`, `set_bk_mode`
  - Primitives: `draw_line`, `draw_rect`, `fill_rect`, `draw_ellipse`, `draw_text`, `draw_text_in_rect`, `draw_bitmap`
  - Query: `text_extent(text) -> (w, h)`
  - Handle: `handle() -> isize` (raw `HDC`), `is_null() -> bool`
- `BackgroundMode { Opaque, Transparent }` — `OPAQUE=2`, `TRANSPARENT=1`
- `PaintDC` — `unsafe fn new(hwnd)` calls `BeginPaint`; `Drop` calls `EndPaint`.
  Used inside `WM_PAINT` handlers.
- `ClientDC` — `new(hwnd)` does `GetDC` + `GetClientRect`; `Drop` calls `ReleaseDC`.
  Draws only in the client area.
- `WindowDC` — `new(hwnd)` does `GetDC` + `ReleaseDC` (regular pair, **not**
  `GetWindowDC`). Lets you paint the title bar / non-client area; almost never
  needed in practice.
- `MemoryDC` — `new()` calls `CreateCompatibleDC` with a 1×1 default bitmap;
  `select_bitmap(&Bitmap)` swaps in the caller's bitmap; `Drop` restores the
  default bitmap then `DeleteDC`s. Used for offscreen rendering and for
  blitting to the screen.

## Public API
```rust
pub trait Dc {
    fn handle(&self) -> isize;
    fn is_null(&self) -> bool { self.handle() == 0 }
    fn set_pen(&mut self, pen: Option<&Pen>);
    fn set_brush(&mut self, brush: Option<&Brush>);
    fn set_text_color(&mut self, colour: Colour);
    fn set_bk_color(&mut self, colour: Colour);
    fn set_bk_mode(&mut self, mode: BackgroundMode);
    fn draw_line(&mut self, x1: i32, y1: i32, x2: i32, y2: i32);
    fn draw_rect(&mut self, x: i32, y: i32, w: i32, h: i32);
    fn fill_rect(&mut self, x: i32, y: i32, w: i32, h: i32, colour: Colour);
    fn draw_ellipse(&mut self, x: i32, y: i32, w: i32, h: i32);
    fn draw_text(&mut self, text: &str, x: i32, y: i32);
    fn draw_text_in_rect(&mut self, text: &str, rect: Rect, center: bool);
    fn draw_bitmap(&mut self, bmp: &Bitmap, x: i32, y: i32);
    fn text_extent(&self, text: &str) -> (i32, i32);
}
```

## Quick start

```rust,no_run
use ru_wx::prelude::*;

// 1. PaintDC — use INSIDE a WM_PAINT handler (the Drop pairs with EndPaint).
let frame_for_paint = frame.clone();
frame.on_paint(move |hwnd| {
    // SAFETY: BeginPaint is only valid inside the WndProc that received
    //         WM_PAINT; the on_paint callback runs in that context.
    let mut dc = unsafe { PaintDC::new(hwnd) };

    // 2. State setters (Pen / Brush / colours / text-mode):
    let pen   = Pen::solid(Colour::BLACK);
    let brush = Brush::solid(Colour::from_rgb(0xEE, 0xEE, 0xEE));
    dc.set_pen(Some(&pen));
    dc.set_brush(Some(&brush));
    dc.set_text_color(Colour::BLUE);
    dc.set_bk_mode(BackgroundMode::Transparent);

    // 3. Primitives:
    dc.draw_line(0, 0, 200, 200);
    dc.draw_rect(10, 10, 100, 50);
    dc.fill_rect(0, 50, 100, 50, Colour::RED);
    dc.draw_ellipse(120, 10, 80, 80);
    dc.draw_text_in_rect("Hello", Rect::xywh(10, 60, 200, 20), true);

    // 4. Bitmap blit (one-shot, uses a transient MemoryDC internally).
    let bmp = Bitmap::new(32, 32);
    dc.draw_bitmap(&bmp, 250, 10);

    // 5. Text metrics for layout:
    let (w, h) = dc.text_extent("Hello");
    println!("extent = {w} x {h}");
});

// 6. ClientDC — paint outside WM_PAINT (e.g. from a button handler).
let frame_for_dc = frame.clone();
some_button.on_click(move |_| {
    let mut dc = ClientDC::new(frame_for_dc.hwnd());
    dc.draw_text("painted on click", 20, 20);
});

// 7. MemoryDC — offscreen rendering to your own Bitmap.
let mut mem = MemoryDC::new();
let target = Bitmap::new(64, 64);
mem.select_bitmap(&target);
mem.set_brush(Some(&Brush::solid(Colour::GREEN)));
mem.fill_rect(0, 0, 64, 64, Colour::GREEN);
// Drop automatically deselects the bitmap and DeleteDC's the memory DC.
```

The four DC types have **identical** drawing methods (they all implement `Dc`). The difference is purely the lifecycle: `PaintDC` pairs `BeginPaint`/`EndPaint`, `ClientDC` pairs `GetDC`/`ReleaseDC`, `WindowDC` does the same but on the non-client area, and `MemoryDC` is `CreateCompatibleDC`/`DeleteDC` with a swappable bitmap.

## Win32 / platform notes
- `draw_text_in_rect` builds format flags: `0x100 | 0x20` (`DT_NOCLIP | DT_SINGLELINE`),
  adds `0x1 | 0x4` (`DT_CENTER | DT_VCENTER`) when `center == true`. Pass
  single-line strings; multi-line text uses the simpler `draw_text` + offset
  pattern.
- `fill_rect` does not require a pre-set brush; it creates a transient
  `CreateSolidBrush(colour)` per call, then `DeleteObject`s it. (Cheap, but
  not allocation-free for tight loops — cache a `Brush` for hot paths.)
- `draw_bitmap` uses a transient `MemoryDC` + `SelectObject` + `BitBlt` with
  `SRCCOPY`. The `MemoryDC` is dropped (and the bitmaps deselected) at the
  end of the call.
- `to_wide_null(s)` helper: `s.encode_utf16().chain(once(0)).collect()` —
  the `DrawTextW` / `TextOutW` length is the `Vec` length minus one (the NUL).
- The four `Dc` impls have identical method bodies (no `&mut self`
  delegation to a shared helper exists). The comment in the file notes the
  duplication is intentional; expect the trait to be the abstraction layer.

## Tests (3)
- `background_mode_round_trip` — `Opaque` / `Transparent` round-trip through
  `bg_mode()`.
- `wide_null_terminates` — `to_wide_null("hi")` produces `[h, i, 0]`.
- `memorydc_smoke_test` — a `MemoryDC` constructs and is non-null on Windows
  (skipped on non-Windows).

## Cross-references
- `pen.rs` / `brush.rs` / `font.rs` — the GDI objects passed to `set_*` /
  used to draw.
- `bitmap.rs` — argument to `draw_bitmap` and to `MemoryDC::select_bitmap`.
- `frame.rs` `WM_PAINT` wndproc — typical site for `PaintDC::new(hwnd)`.

## Example
```rust,no_run
use ru_wx::prelude::*;

let frame = Frame::builder().with_title("dc").build();
frame.on_paint(|hwnd| {
    let mut dc = unsafe { PaintDC::new(hwnd) };
    dc.set_bk_mode(BackgroundMode::Transparent);
    dc.draw_text_in_rect("hello", Rect::xywh(10, 10, 200, 20), false);
});
```
