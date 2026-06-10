# message_dialog.rs

Class-based wrapper around [`message_box`](message_box.md). Use when the prompt's title / message / style / icon are known up front and you want a single value object that you can show with `show_modal()`.

## Purpose
A small, configurable `MessageBox` surrogate. The constructor takes a `&Frame` parent (required for proper Z-ordering and modality), and the four fields can be tweaked via setters before `show_modal` is called.

## Key Types
- `MessageDialog<'a>` — public struct, lifetime-tied to the parent `Frame`. Fields: `parent: &'a Frame`, `title: String`, `message: String`, `style: MessageBoxStyle`, `icon: MessageBoxIcon`.
- `MessageDialogStyle` — type alias for [`MessageBoxStyle`](message_box.md).
- `MessageDialogIcon` — type alias for [`MessageBoxIcon`](message_box.md).

## Key Methods
- `MessageDialog::new(parent: &'a Frame, title: &str, message: &str, style: MessageBoxStyle, icon: MessageBoxIcon) -> Self` — Build a configured dialog.
- `set_message(&mut self, message: &str)` / `message(&self) -> &str` — getters/setters.
- `set_title(&mut self, title: &str)` / `title(&self) -> &str`.
- `set_style(&mut self, style: MessageBoxStyle)`.
- `set_icon(&mut self, icon: MessageBoxIcon)`.
- `show_modal(&self) -> MessageBoxResult` — Windows-only. Delegates to `message_box` with the current fields.

## Win32 Notes
- Implementation is a thin wrapper over `MessageBoxW` (via `message_box`). No new Win32 surface.
- The `'a` lifetime on the parent frame is required to express the modal-block-on-parent relationship; cloning the `MessageDialog` is not allowed (the parent must outlive it).

## Quick start

```rust
use ru_wx::prelude::*;

let r = MessageDialog::new(
    &frame,
    "Confirm",                          // title
    "Apply changes?",                  // message
    MessageBoxStyle::YesNo,
    MessageBoxIcon::Question,
).show_modal();

if matches!(r, MessageBoxResult::Yes) { /* apply */ }
```

Tweak the dialog fields via `set_message` / `set_title` / `set_style` / `set_icon` before calling `show_modal()`.

## See Also
- [`message_box.rs`](message_box.md) — the underlying free function
- [`dialog.rs`](../window/dialog.md) — full custom dialog when you need more than a simple prompt
