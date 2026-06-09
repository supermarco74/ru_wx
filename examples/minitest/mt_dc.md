# mt_dc.rs

The "kitchen-sink" minitest for the [`wxDC`](file:///f:/code/ru_wx/ru_wx/src/dc.rs) family — every public DC flavour in `ru_wx`, plus `Pen` / `Brush` / `BackgroundMode`.

**Run:** `cargo run --example mt_dc`. Close the window to exit.

## Purpose
Demonstrate every public DC flavour in `ru_wx`:
- `Pen` / `Brush` / `BackgroundMode` construction
- `Pen::solid` / `Brush::solid` convenience constructors
- [`MemoryDC`](file:///f:/code/ru_wx/ru_wx/src/dc.rs) + [`Bitmap`](file:///f:/code/ru_wx/ru_wx/src/bitmap.rs) backing store (full [`Dc`] trait: lines, rects, ellipses, text, blit)
- [`ClientDC`](file:///f:/code/ru_wx/ru_wx/src/dc.rs) — transient drawing on the client area (drag feedback, overlay annotations, debug reticles)
- [`WindowDC`](file:///f:/code/ru_wx/ru_wx/src/dc.rs) — transient drawing on the **whole** window (client + non-client)
- `Frame::register_paint_handler` arm — receives the live `HDC` from the `BeginPaint` / `EndPaint` cycle

> ⚠️ `PaintDC` is **not** constructed here on purpose: it would call `BeginPaint` a second time — Win32 undefined behaviour. `PaintDC::new` is exercised in the unit tests inside `src/dc.rs` instead.

## Top-level flow
1. Frame 800×600.
2. **Section 1** — pre-build `mem_bmp: Bitmap::new(220, 220)` outside the closure so the paint handler can borrow it for the lifetime of the message loop.
3. **Section 2** — `Pen::new(...)` / `Brush::new(...)` / `Pen::solid(...)` / `BrushStyle::Transparent` round-trip, then read back `.colour`, `.width`, `.style`.
4. **Section 3** — `MemoryDC` block:
   - `set_bk_mode(BackgroundMode::Opaque)`
   - `set_bk_color(Colour::WHITE)` + `fill_rect(0,0,220,220,Colour::WHITE)`
   - `set_pen(Some(&red_pen))`, `set_brush(Some(&blue_brush))`
   - `draw_rect(10,10,200,200)`, `draw_ellipse(30,30,160,160)`
   - `set_brush(None)` → `draw_ellipse(60,60,100,100)` (outline only)
   - `draw_line(...)` diagonal cross
   - `set_pen(Some(&green_pen))`, `set_text_color(Colour::BLACK)`
   - `text_extent("MemoryDC")` → `draw_text` centred
   - `draw_text_in_rect("drew into a bitmap", Rect::new(10,170,200,30), true)`
5. **Section 4** — `ClientDC::new(frame.hwnd())` block:
   - `handle()`, `is_null()` round-trips
   - `set_bk_mode(Transparent)`, `set_text_color(green)`
   - `draw_text("ClientDC: transient draw (released in Drop)", 240, 10)`
   - `Pen::solid(green)` → diagonal reticle
6. **Section 5** — `WindowDC::new(frame.hwnd())` block:
   - `fill_rect(0,0,16,16, …)` top-left corner
   - `fill_rect(0,0,800,4, …)` top edge
   - `fill_rect(0,0,4,600, …)` left edge
7. **Section 6** — paint handler:
   - `let mem_bmp_alive = mem_bmp.width;` — copy the `u32` (a `'static` number) so the `FnMut + 'static` closure can compile
   - `frame.register_paint_handler(move |hdc: isize| { let _ = (hdc, mem_bmp_alive); });` — closure shape, no `BitBlt` (that needs raw FFI)

## Key APIs exercised
| Type | Calls |
|---|---|
| `Pen` | `new(colour, width, style)`, `solid(colour)`, `.colour`, `.width`, `.style` |
| `Brush` | `new(colour, style)`, `.colour`, `.style` |
| `BackgroundMode` | `Opaque`, `Transparent` |
| `PenStyle` / `BrushStyle` | `Solid`, `Transparent` |
| `MemoryDC` | `new`, `select_bitmap(&bmp)`, `set_bk_mode`, `set_bk_color`, `set_pen`, `set_brush`, `set_text_color`, `fill_rect`, `draw_rect`, `draw_ellipse`, `draw_line`, `draw_text`, `draw_text_in_rect`, `text_extent` |
| `ClientDC` | `new(hwnd)`, `handle`, `is_null`, `set_bk_mode`, `set_text_color`, `draw_text`, `set_pen`, `set_brush`, `draw_line`, `draw_rect` |
| `WindowDC` | `new(hwnd)`, `handle`, `is_null`, `fill_rect` |
| `Bitmap` | `new(w, h)` — 32-bit DIB section |
| `Rect` | `new(x, y, w, h)` |
| `Colour` | `new`, `BLACK`, `WHITE` |
| `Frame` | `register_paint_handler(FnMut + 'static)` |

## Patterns worth noting
- **`'static` workaround** — `register_paint_handler` needs `FnMut + 'static`, so the closure cannot borrow `mem_bmp` directly. The test extracts a `'static` value (`mem_bmp.width: u32`) as a compile-time proof that the bitmap is still alive when the paint fires.
- **`Drop` releases the DC** — both `ClientDC` and `WindowDC` are scoped inside `{}` blocks so the destructor runs before the message loop, releasing the Win32 `HDC`.
- **`set_brush(None)`** is the only way to draw an unfilled outline; setting a `Brush::new(BLACK, Transparent)` style brush is also possible but the explicit `None` keeps the code intent clear.

## Win32 notes
- All `*_DC` types wrap an `HDC`; `select_bitmap` calls `SelectObject` and stores the previous handle for restoration on `Drop`.
- The 32-bit DIB section used by `MemoryDC` has `DIB_RGB_COLORS` + `BI_RGB` and is top-down — ru_wx's `Image` swizzles BGRA→RGBA on the way in.
- `draw_text` uses `SetTextColor` + `ExtTextOutW`; `draw_text_in_rect` adds `DT_CENTER | DT_VCENTER | DT_SINGLELINE`.
- `WindowDC` calls `GetWindowDC(hwnd)`; `ClientDC` calls `GetDC(hwnd)` — the former covers the title bar, the latter doesn't.
- The paint handler closure receives the live `HDC` (an `isize` in ru_wx's wrapper) so callers can `BitBlt` / `StretchBlt` it via raw FFI without going through `PaintDC::new`.

## Cross-references
- [`dc.md`](file:///f:/code/ru_wx/ru_wx/src/dc.md) — `Dc` trait, all DC types
- [`pen.md`](file:///f:/code/ru_wx/ru_wx/src/pen.md), [`brush.md`](file:///f:/code/ru_wx/ru_wx/src/brush.md)
- [`bitmap.md`](file:///f:/code/ru_wx/ru_wx/src/bitmap.md)
- [`rect.md`](file:///f:/code/ru_wx/ru_wx/src/geometry.md#rect) (geometry module)
- [`colour.md`](file:///f:/code/ru_wx/ru_wx/src/colour.md)
- [`frame.md`](file:///f:/code/ru_wx/ru_wx/src/frame.md) — `register_paint_handler`
