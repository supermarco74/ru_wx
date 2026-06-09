# text_ctrl.rs

Single- or multi-line text editor (`wxTextCtrl`) on Windows — `EDIT` common control.

## Purpose
Provides three flavours:
- `TextCtrl::new` — single-line input, `ES_AUTOHSCROLL`.
- `TextCtrl::multiline` — multi-line input, `ES_MULTILINE | ES_AUTOVSCROLL | ES_WANTRETURN | WS_VSCROLL`. **No** `ES_AUTOHSCROLL` (its presence disables word-wrap).
- `TextCtrl::password` — single-line with `ES_PASSWORD` (masked characters).

CRLF normalisation: `\n` from the caller is converted to `\r\n` on the way in (and back to `\n` on the way out) so multiline edits behave correctly on Win32. Existing `\r\n` pairs are preserved (no double-conversion).

## Key Types
- `TextCtrl` — `Clone`, wraps `Rc<RefCell<TextCtrlInner>>`. `TextCtrlInner` holds `hwnd`, `id`, `rect`, `multiline`, `enabled`, `visible`, cached `readonly`, cached `max_length`.

## Key Functions/Methods
- `TextCtrl::new<W: Window>(parent, default_text)` — 150×24 single-line.
- `TextCtrl::multiline<W: Window>(parent, default_text)` — 200×100 multiline. Seeds the text post-creation with `SetWindowTextW` (passing CRLF-separated text via `lpWindowName` at `CreateWindowExW` time is unreliable for multi-line).
- `TextCtrl::password<W: Window>(parent, default_text)` — 150×24 masked.
- `TextCtrl::get_value(&self) -> String` — `GetWindowTextLengthW` + `GetWindowTextW`; multiline values have `\r\n` collapsed back to `\n`.
- `TextCtrl::set_value(&self, text)` — `SetWindowTextW` with CRLF normalisation.
- `TextCtrl::set_readonly(&self, bool)` / `is_readonly(&self) -> bool` — `EM_SETREADONLY` (0x00CF), cached.
- `TextCtrl::set_max_length(&self, max: u32)` / `max_length(&self) -> u32` — `EM_SETLIMITTEXT` (0x00C5); 0 = unlimited. Truncation is the user's problem; existing text is not erased.
- `TextCtrl::clear(&self)` — `SetWindowTextW(hwnd, null)`.
- `TextCtrl::append_text(&self, text)` — `EM_SETSEL(-1, -1)` (move caret to end) then `EM_REPLACESEL` (0x00C2).
- `TextCtrl::can_undo(&self) -> bool` / `undo(&self)` — `EM_CANUNDO` (0x00C6) / `WM_UNDO` (0x0304).
- `TextCtrl::on_change<F>(&self, frame, cb)` — registers a handler on the frame's command-handler map (fires for `EN_CHANGE`).
- `TextCtrl::id(&self) -> u16`, `TextCtrl::as_widget_ref(&self) -> WidgetRef`.

## Win32 Notes
- Class `EDIT`.
- Single-line: `WS_CHILD | WS_VISIBLE | WS_BORDER | ES_AUTOHSCROLL`.
- Multi-line: `WS_CHILD | WS_VISIBLE | WS_BORDER | ES_MULTILINE | ES_AUTOVSCROLL | ES_WANTRETURN | WS_VSCROLL` (no `ES_AUTOHSCROLL` so wrapping works).
- Password: `WS_CHILD | WS_VISIBLE | WS_BORDER | ES_PASSWORD | ES_AUTOHSCROLL`.
- `ES_PASSWORD = 0x0020` masks input with `*`.
- `ES_WANTRETURN = 0x1000` lets Enter insert a newline instead of activating the default dialog button.
- `ES_READONLY = 0x0800` is exposed as the `EM_SETREADONLY` toggle, not as a creation style here.
- `EM_SETSEL` with `wparam = -1isize as usize` and `lparam = -1isize` collapses the selection to the caret at end-of-buffer.
- `EM_REPLACESEL` `lparam` is a pointer to a UTF-16 null-terminated buffer.

## Quick start

```rust
use ru_wx::prelude::*;

// frame is the owning Frame.
let single = TextCtrl::new(&frame, "default value");
let multi  = TextCtrl::multiline(&frame, "line 1\nline 2");
let pwd    = TextCtrl::password(&frame, "");

// Read / write.
single.set_value("hello");
let v = single.get_value();

// Append to a multiline (carrot is moved to end first).
multi.append_text("\nanother line");

// React to every change.
let label = StaticText::new(&frame, "");
let single_for_cb = single.clone();
let label_for_cb = label.clone();
single.on_change(&frame, move || {
    label_for_cb.set_label(&single_for_cb.get_value());
});
```

`\n` is normalised to `\r\n` on the way in (and back to `\n` on the
way out) so multi-line edits behave correctly on Win32. The single-line
variant uses `ES_AUTOHSCROLL` (so it doesn't wrap); the multi-line
variant omits it (so wrapping works).

## See Also
- [`combo_box.rs`](./combo_box.md) — text input with drop-down list.
- [`frame.rs`](./frame.md) — `register_command_handler` used by `on_change`.
- [`widget.rs`](./widget.md) — `Widget` trait.
