# static_text.rs

Read-only text label widget (`wxStaticText` analog). Displays a single line of left-aligned text that the user cannot edit.

## Purpose
A non-interactive label control. Used for captioning form fields, headers, status messages, or any static UI text. Single-line; for multi-line wrapped text, see `TextCtrl` (read-only) instead.

## Key Types
- `StaticText` — public struct holding the platform handle via `*Inner`.
- `StaticTextInner` (private) — Win32 `HWND` for the underlying `STATIC` control.

## Key Methods
- `StaticText::new<W: Window>(parent_in: &W, text: &str) -> Self` — Creates a 200×20 left-aligned `STATIC` child. Parent is any `Window` (typically a `Panel`, `Frame`, or `Dialog`).
- `set_label(&self, text: &str)` — Calls `SetWindowTextW`.
- `get_label(&self) -> String` — Reads text back via `GetWindowTextLengthW` + `GetWindowTextW`.
- `set_font(&self, font: &Font)` — Sends `WM_SETFONT` with `lparam=1` to redraw immediately.
- `is_enabled() -> bool` — Always returns `true`; static text is always enabled by design.
- `set_enabled(&self, _enabled: bool)` — No-op; provided to satisfy the `Widget` trait but has no visual effect.

## Win32 Notes
- Window class: built-in `STATIC`.
- Styles: `WS_CHILD | WS_VISIBLE | SS_LEFT`. Left-aligned single line.
- No custom WndProc — delegates to the default `STATIC` control. Because the control never generates `WM_COMMAND` notifications for clicks, this widget does **not** participate in the frame's command-handler dispatch table.
- `is_enabled` / `set_enabled` exist only for trait uniformity. The Win32 `SS_LEFT` style does not honour `WS_DISABLED` for text colour; ru_wx intentionally exposes no disabled state.

## Quick start

```rust
use ru_wx::prelude::*;

let label = StaticText::new(&frame, "Hello, world!");

// Update the text at runtime:
label.set_label("Updated!");

// Read it back:
let s = label.get_label();
```

A `StaticText` is single-line. For read-only multi-line text, use a read-only `TextCtrl` instead.

## See Also
- [`button.rs`](button.md) — interactive control with a label
- [`text_ctrl.rs`](text_ctrl.md) — read-only multi-line alternative
- [`static_box.rs`](static_box.md) — labelled border container
