# find_replace_dialog.rs

Find / Replace dialog (`wxFindReplaceDialog`).

## Purpose
A modeless (non-modal) dialog that hosts a standard "find" or "find and replace" panel. The user can keep the dialog open while interacting with the rest of the application. Find / replace notifications are delivered through [`FindReplaceEvent`] values that the caller polls (or pulls) from the dialog.

## Key Types
- `FindReplaceDialog` — owns the dialog state and the current find / replace flags.
- `FindReplaceEvent` — `enum` of the user actions: `Find`, `Replace`, `ReplaceAll`, `Close`.

## Key Functions/Methods
- `FindReplaceDialog::new<W: Window>(parent: &W, flags: FindReplaceFlags, find_text: &str, replace_text: &str, caption: &str)` — constructs a modeless dialog.
- `FindReplaceDialog::show(show: bool)` — show or hide the dialog window.
- `FindReplaceDialog::poll_event(&self) -> Option<FindReplaceEvent>` — non-blocking poll; returns the most recent event if any, otherwise `None`.
- `FindReplaceDialog::find_text(&self) -> String` — read the current "find" text.
- `FindReplaceDialog::replace_text(&self) -> String` — read the current "replace" text.
- `FindReplaceDialog::update(&self)` — pump pending messages so notifications are delivered.

## Win32 Notes
- Wraps the `comdlg32!FindTextW` / `ReplaceTextW` API.
- Uses a `FINDREPLACEW` struct on the stack as the dialog's working memory.
- Modeless: the dialog creates its own message loop, the caller's thread is not blocked.
- All FFI calls wrapped in `// SAFETY:` comments documenting validated arguments.

## Quick start

```rust
use ru_wx::prelude::*;

let dlg = FindReplaceDialog::new(
    &frame,
    FindReplaceFlags::default(),
    "old",            // initial find text
    "new",            // initial replace text
    "Find / Replace", // caption
);
dlg.show(true);

// In your frame's update / idle handler:
if let Some(ev) = dlg.poll_event() {
    match ev {
        FindReplaceEvent::Find       => { /* search dlg.find_text() */ }
        FindReplaceEvent::Replace    => { /* replace one */ }
        FindReplaceEvent::ReplaceAll => { /* replace all */ }
        FindReplaceEvent::Close      => { /* dialog closed */ }
    }
}
```

`poll_event` is **non-blocking**; call it from a `Timer` or `update()` loop.

## See Also
- [`message_dialog.rs`](message_dialog.md) — modal sibling dialog pattern (for comparison).
- [`text_entry_dialog.rs`](text_entry_dialog.md) — modal sibling dialog pattern.
