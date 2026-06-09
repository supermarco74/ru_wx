# button.rs

Push-button control mapped to Win32 `BUTTON` class with style `BS_PUSHBUTTON`.

## Purpose
Wraps a native Win32 push-button. Supports plain text labels, programmatic bitmaps, and SVG icons (loaded from file or embedded bytes). The button is a child of a `Frame` or any `Window`, and click events are dispatched via the frame's command handler map.

## Key Types
- `Button` — `Clone`, holds `Rc<RefCell<ButtonInner>>`. `ButtonInner` stores `hwnd: HWND`, `id: u16`, `label`, `rect`, `enabled`, `visible`, optional `hbitmap: HBITMAP`.

## Key Functions/Methods
- `Button::new<W: Window>(parent, label)` — creates a default text button (100×30 px).
- `Button::GetDefaultSize() -> (i32, i32)` — static method mirroring `wxButton::GetDefaultSize`. Returns the platform's default button size in pixels: `(88, 26)` on Windows, `(75, 23)` on other platforms.
- `Button::new_with_bitmap<W: Window>(parent, label, colour, w, h)` — creates a button and attaches a solid-colour bitmap via `BM_SETIMAGE`. Uses GDI: `CreateCompatibleDC` → `CreateCompatibleBitmap` → `CreateSolidBrush(colour.to_colorref())` → `FillRect`.
- `Button::new_with_svg_icon<W: Window>(parent, svg_path, icon_size)` — rasterises SVG file to `icon_size × icon_size` and attaches it.
- `Button::new_with_svg_bytes<W: Window>(parent, svg_bytes, icon_size)` — same but from `&[u8]` (e.g. `include_bytes!`).
- `Button::on_click<F: FnMut() + 'static>(&self, frame: &Frame, cb)` — registers click handler on the frame's command-handler map keyed by control id.
- `Button::set_label(&self, label)` / `Button::get_label(&self) -> String` — `SetWindowTextW` / `GetWindowTextW`.
- `Button::id(&self) -> u16` — returns the control id used for `WM_COMMAND` dispatch.
- `Button::as_widget_ref(&self) -> WidgetRef` — for use with sizers.

## Win32 Notes
- `BUTTON` class, `BS_PUSHBUTTON` (default) or `BS_BITMAP` for image buttons.
- Sends `BM_SETIMAGE` (`0x00F7`) with `IMAGE_BITMAP = 0`.
- Default size 100×30 (overridden by sizer).
- `Drop` calls `DeleteObject(hbitmap)` to release GDI bitmap handle.
- All FFI calls wrapped in `// SAFETY:` comments documenting validated arguments.

## Quick start

```rust
use ru_wx::prelude::*;

// frame is the owning Frame.
let button = Button::new(&frame, "Click me");
let label  = StaticText::new(&frame, "Idle");

// Closure must be 'static + Send-able; clone the shared widget for it.
let label_for_click = label.clone();
button.on_click(&frame, move || {
    label_for_click.set_label("Button clicked!");
});

// Optional: icon button (SVG bytes embedded at compile time).
let icon_btn = Button::new_with_svg_bytes(
    &frame,
    include_bytes!("../assets/icons/star.svg"),
    24,                          // raster size (px)
);
```

The button is a child of the `Frame`; the click handler is stored on the
frame's `WM_COMMAND` dispatch table, keyed by `button.id()`. Pair with a
[`BoxSizer`](./sizer.md) to manage size and alignment.

## See Also
- [`frame.rs`](./frame.md) — `Frame::register_command_handler` used by `on_click`.
- [`geometry.rs`](./geometry.md) — `Colour`, `Rect`, `Colour::to_colorref()`.
- [`icon.rs`](./icon.md) — `load_svg_as_hbitmap`, `svg_bytes_to_hbitmap`.
- [`widget.rs`](./widget.md) — `Widget` trait, `Window` trait, `WidgetRef`.
- [`platform/win32.rs`](./platform/win32.md) — `next_control_id`, `to_wide`.
