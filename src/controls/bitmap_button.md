# bitmap_button.rs

Bitmap-button control mapped to Win32 `BUTTON` class with style `BS_BITMAP`.

## Purpose
Wraps a native Win32 bitmap button. The button displays a bitmap instead of a text label. Up to four state bitmaps can be attached: **label**, **selected**, **disabled**, and **focus**. The button is a child of a `Frame` or any `Window`; click events are dispatched via the frame's command handler map.

## Key Types
- `BitmapButton` — `Clone`, holds `Rc<RefCell<BitmapButtonInner>>`. `BitmapButtonInner` stores `hwnd: HWND`, `id: u16`, `rect`, `enabled`, `visible`, `bmp_width`, `bmp_height`, and four `isize` bitmap handles (one per state).

## Key Functions/Methods
- `BitmapButton::new<W: Window>(parent, bitmap: &Bitmap, width, height)` — creates a bitmap button at `width × height` pixels with the given bitmap attached as the label.
- `BitmapButton::new_from_svg<W: Window>(parent, svg_path, width, height)` — rasterises the SVG to `width × height` and attaches it.
- `BitmapButton::new_from_svg_bytes<W: Window>(parent, svg_bytes, width, height)` — same but from `&[u8]` (e.g. `include_bytes!`).
- `BitmapButton::set_bitmap_label(&self, bitmap: &Bitmap)` — replaces the label bitmap at runtime; the old bitmap is freed via `DeleteObject`.
- `BitmapButton::bitmap_width(&self) -> i32` / `bitmap_height(&self) -> i32` — return the bitmap dimensions.
- `BitmapButton::on_click<F: FnMut() + 'static>(&self, frame: &Frame, cb)` — registers a click handler.
- `BitmapButton::id(&self) -> u16` — returns the control id used for `WM_COMMAND` dispatch.
- `BitmapButton::as_widget_ref(&self) -> WidgetRef` — for use with sizers.

## Win32 Notes
- `BUTTON` class, `BS_BITMAP` (`0x0080`) style.
- Sends `BM_SETIMAGE` (`0x00F7`) with `IMAGE_BITMAP = 0` to attach the bitmap.
- The control returns the previous bitmap when `BM_SETIMAGE` is called; we `DeleteObject` that previous bitmap so the user can safely replace it any number of times.
- `Drop` calls `DeleteObject` on every non-zero state bitmap to release GDI handles.
- All FFI calls wrapped in `// SAFETY:` comments documenting validated arguments.

## Quick start

```rust
use ru_wx::prelude::*;

// frame is the owning Frame.

// From a runtime-allocated Bitmap.
let bmp  = Bitmap::new_solid(64, 64, Colour::new(0, 120, 200, 255));
let btn  = BitmapButton::new(&frame, &bmp, 64, 64);

// Or from an SVG file / bytes.
let svg_btn = BitmapButton::new_from_svg(
    &frame,
    "assets/icons/star.svg",
    48, 48,
);

let icon_btn = BitmapButton::new_from_svg_bytes(
    &frame,
    include_bytes!("../assets/icons/star.svg"),
    32, 32,
);

// Wire a click handler just like a regular Button.
svg_btn.on_click(&frame, || { /* … */ });
```

The bitmap replaces the text label entirely. To swap the image at
runtime, call `set_bitmap_label(&self, &bmp)` — the previous bitmap is
freed automatically.

## See Also
- [`button.rs`](button.md) — sibling `Button` (text + bitmap) for comparison.
- [`bitmap.rs`](../dc/bitmap.md) — `Bitmap` type used here.
- [`icon.rs`](../dc/icon.md) — `load_svg_as_hbitmap`, `svg_bytes_to_hbitmap`.
- [`widget.rs`](../core/widget.md) — `Widget` trait, `Window` trait, `WidgetRef`.
- [`platform/win32.rs`](../platform/win32.md) — `next_control_id`, `to_wide`.
