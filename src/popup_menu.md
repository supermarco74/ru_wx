# `popup_menu.rs` — context-menu wrapper around `Menu`

A thin convenience wrapper around [`Menu`](menu.md) for the right-click
"context menu" use case. The only addition over `Menu` is the
`popup`/`popup_at` helpers, which call `TrackPopupMenu` for you.

## Purpose

The shape of the public API is identical to `Menu` for the *building* phase
(`append`, `append_disabled`, `append_check_item`, `append_radio_item`,
`append_separator`, `append_with_*_icon`, `check_item`). The new methods are
the two `popup` variants that show the menu.

This is purely a readability win — using `PopupMenu` makes the call site say
"I'm popping up a context menu", not "I'm building a Menu and showing it".

## Public type

```rust
#[derive(Clone)]
pub struct PopupMenu { /* Rc<RefCell<Menu>> */ }
```

## Public API

| Method | Purpose |
|---|---|
| `new()` | Create an empty popup menu. |
| `append(label, frame, F)` | Normal item with click callback. |
| `append_disabled(label)` | Greyed-out item. |
| `append_check_item(label, frame, F) -> u16` | Checkable item. |
| `append_radio_item(label, frame, F) -> u16` | Radio item. |
| `append_separator()` | Horizontal divider line. |
| `append_with_colour_icon(label, fg, bg, F)` | Item with colour-icon. |
| `append_with_svg_icon(label, svg, F)` | Item with SVG icon. |
| `check_item(id, check) -> bool` | Toggle the check state. |
| `popup(frame)` | Show the menu at the current cursor position. |
| `popup_at(frame, x, y)` | Show the menu at explicit `(x, y)` (in `frame` client coords). |
| `as_menu() -> &Menu` | Borrow the inner `Menu`. |
| `as_menu_mut() -> &mut Menu` | Mutable borrow of the inner `Menu`. |

## Win32 notes

- `popup` delegates to `Menu::popup_at_cursor(frame.hwnd())`.
- `popup_at` calls **`SetForegroundWindow`** + **`TrackPopupMenu`** with
  flags `TPM_BOTTOMALIGN | TPM_RIGHTBUTTON` and then posts a
  `WM_NULL` via `PostMessageW` to force-close any sticky popup state.
- The frame's HWND is used as the *owner* so the popup is dismissed
  automatically when the frame is destroyed.
- All append methods are inline `&mut self` delegates to the inner `Menu`'s
  methods; the menu's command id namespace is shared with the frame's
  `register_command_handler` table.

## Quick start

```rust
use ru_wx::prelude::*;

// frame is the owning Frame (e.g. a Panel as well).
let popup = PopupMenu::new();

// 1) Build items; each click callback is registered against the frame's
//    command-id table, so we keep a `frame_for_*` clone for the closure.
let frame_for_cut   = frame.clone();
let frame_for_copy  = frame.clone();
let frame_for_paste = frame.clone();

popup.append("Cut",   &frame, move || println!("cut"));
popup.append("Copy",  &frame, move || println!("copy"));
popup.append("Paste", &frame, move || println!("paste"));
popup.append_separator();
popup.append_disabled("Disabled item");

// 2) Show the menu at the current cursor position (right-click context).
//    `popup_at(frame, x, y)` is the same but uses explicit client coords.
popup.popup(&frame);
```

The menu is dismissed automatically when the frame is destroyed
because the frame's HWND is the popup owner.

## Cross-references

- [`menu.md`](menu.md) — the inner type and the rest of the menu API.
- [`frame.md`](frame.md) — needed as an argument to `popup*` and to all
  `append_*` variants that take a callback (because the callback is
  registered against the frame's command-id table).
