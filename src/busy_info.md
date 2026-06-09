# busy_info.rs

Non-modal "busy" overlay (`wxBusyInfo`).

## Purpose
Displays a small text label with a busy cursor, typically centered on a parent window, while a long-running operation is in progress. Unlike a modal dialog, `BusyInfo` does not block the calling thread — the caller is expected to drop the `BusyInfo` value when the operation finishes.

## Key Types
- `BusyInfo` — `Drop` is the primary API: dropping the value hides the label window.

## Key Functions/Methods
- `BusyInfo::new<W: Window>(parent: &W, message: &str)` — creates a centered label with the given message and shows the `IDC_WAIT` cursor on the parent.
- `BusyInfo::update_message(&self, msg: &str)` — change the displayed message at runtime.
- `message(&self) -> &str` — read the current message.
- `update(&self)` — pump pending messages so the label repaints during long-running operations.

## Win32 Notes
- Implemented as a `STATIC` child window positioned in the center of the parent.
- Sets the parent window's cursor to `IDC_WAIT` for the lifetime of the `BusyInfo`.
- All FFI calls wrapped in `// SAFETY:` comments documenting validated arguments.

## Quick start

```rust
use ru_wx::prelude::*;

fn long_op(parent: &Frame) {
    let _busy = BusyInfo::new(parent, "Working, please wait…");
    // … do the work …
    // `_busy` is dropped at the end of this scope, hiding the overlay
    // and restoring the parent's cursor to IDC_ARROW.
}
```

For non-`Frame` parents, any `Window` works (e.g. a `Panel`).

## See Also
- [`progress_dialog.rs`](./progress_dialog.md) — modal alternative with a progress bar.
- [`message_dialog.rs`](./message_dialog.md) — modal alternative for short messages.
- [`widget.rs`](./widget.md) — `Window` trait for the parent.
