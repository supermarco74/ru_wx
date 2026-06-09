# `status_bar.rs` — `StatusBar` (msctls_statusbar32 wrapper)

A wrapper around the Win32 **`msctls_statusbar32`** common control, attached
as a child of a `Frame`. The status bar is a single bottom-of-frame strip
divided into `N` text fields.

## Purpose

- A `StatusBar` is always created *attached to a frame*. The frame's sizer
  does **not** manage it — the bar is positioned manually at the bottom of
  the frame's client area, and re-positioned whenever the frame is resized.
- Field widths are split **evenly** across the frame's client width (the
  Win32 default behaviour). There is no per-field-width API in this version.

## Public type

```rust
#[derive(Clone)]
pub struct StatusBar { /* Rc<RefCell<StatusBarInner>> */ }
```

## Public API

| Method | Purpose |
|---|---|
| `new(frame, fields) -> Self` | Create a status bar with `fields` fields attached to `frame`. |
| `set_status_text(&self, text, i)` | Set the text of field `i`. |
| `get_status_text(&self, i) -> String` | Read the text of field `i`. |
| `get_fields_count(&self) -> usize` | Number of fields. |
| `hwnd() -> HWND` (cfg windows) | Raw Win32 handle. |
| `as_widget_ref(&self) -> WidgetRef` | For sizer interop (rare; the bar is usually not sizer-managed). |
| `is_visible(&self) -> bool` | Current visibility flag. |
| `set_visible(&self, visible)` | Show or hide. |

## Quick start

```rust,no_run
use ru_wx::prelude::*;

// 1. Create a 3-field status bar attached to a frame.
let bar = StatusBar::new(&frame, 3);
frame.set_status_bar(&bar);

// 2. Set / read each field by 0-based index.
bar.set_status_text("Ready", 0);
bar.set_status_text("Line 42, Col 7", 1);
bar.set_status_text("UTF-8", 2);
assert_eq!(bar.get_status_text(0), "Ready");
assert_eq!(bar.get_fields_count(), 3);

// 3. Update from anywhere — the bar is Clone, so captured clones are safe.
let bar_for_tick = bar.clone();
bar_for_tick.set_status_text("Working…", 0);
some_button.on_click(move |_| {
    bar_for_tick.set_status_text("Done", 0);
});

// 4. Show / hide.
bar.set_visible(false);   // hide the bar
bar.set_visible(true);    // show it again
```

The bar is **not** sizer-managed; it is manually re-positioned at the bottom of the frame's client area on every `WM_SIZE` via a resize handler installed by `new()`. Field widths are split evenly across the frame's client width. A size-grip area (`SBARS_SIZEGRIP`) is included for resizing the frame from the bottom-right corner.

## Win32 notes

- Constant: **`STATUS_BAR_HEIGHT = 22`** pixels.
- Created via `CreateWindowExW(0, "msctls_statusbar32", ...,
  WS_CHILD | WS_VISIBLE | SBARS_SIZEGRIP, ...)`.
- The size-grip area is enabled with the **`SBARS_SIZEGRIP = 0x0100`** style
  bit (a non-standard style that the `msctls_statusbar32` control understands).
- Per-field text is set with **`SB_SETTEXT = WM_USER + 1 = 0x0401`**, where
  the low word of `wParam` is the field index and `lParam` is the wide-string
  text.
- Field partitioning is done with **`SB_SETPARTS = WM_USER + 4 = 0x0404`**
  (a `*const i32` array of `parts + 1` entries; the last is `-1` for the
  right edge). The read-back uses **`SB_GETPARTS = WM_USER + 6 = 0x0406`**.
- The simple mode (`SB_SIMPLE = 0x0409`) is supported by the underlying
  control but not exposed by the public API in this version.

## The resize re-layout

The bar does **not** sit inside a sizer; instead, `new()` registers a
**resize callback** with the frame via `frame.add_resize_handler(...)`. That
callback fires on every `WM_SIZE` and:

1. Re-computes the field widths as an even split of the frame's current
   client width.
2. Sends `SB_SETPARTS` with the new partition table.
3. Calls `MoveWindow` on the status bar's HWND to re-position it at the
   bottom of the client area.

The handler captures a `Weak<RefCell<StatusBarInner>>` so that destroying
the status bar turns the callback into a no-op (no use-after-free).

## Z-order trick

After the resize handler re-positions the bar, it calls
`SetWindowPos(HWND_TOP, ..., SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE)`.
This is a documented fix for the status bar being painted *over* by
sizer-managed children: the bar needs to be at the top of the Z-order even
though it lives at the bottom of the client area in terms of layout.

## Cross-references

- [`frame.md`](frame.md) — `frame.add_resize_handler` and the resize
  callback signature. `Frame::set_status_bar(&StatusBar)` is the typical
  attachment point (called internally from `new`).
- [`widget.md`](widget.md) — the `Widget` trait implemented by the inner type.
- [`tool_bar.md`](tool_bar.md) — sibling native-control wrapper; both share
  the "no sizer, manual MoveWindow on resize" pattern.
