# text_entry_dialog.rs

Single-line text-entry dialogs (`wxTextEntryDialog`, `wxPasswordEntryDialog`, `wxNumberEntryDialog`).

## Purpose
Three modal popups built from the same skeleton: a `STATIC` label, a single-line `EDIT`, and OK / Cancel buttons. They differ only in the `EDIT` style bits and the return type.

| Type | EDIT style | Return |
|---|---|---|
| `TextEntryDialog`   | plain         | `Option<String>` |
| `PasswordEntryDialog` | `ES_PASSWORD` | `Option<String>` |
| `NumberEntryDialog`   | `ES_NUMBER`   | `Option<i64>` |

## Key Types
- `TextEntryDialog` — wraps an `Rc<RefCell<EntryDialogInner>>` plus `message`, `caption`, `default`.
- `PasswordEntryDialog` — same skeleton, no `default`.
- `NumberEntryDialog` — same skeleton, plus `initial: i64`, `min_value: Option<i64>`, `max_value: Option<i64>`.
- `EntryDialogInner` — stores `hwnd`, `hwnd_label`, `hwnd_edit`, `result: Option<String>`, `is_done: bool`, `kind: EditStyle` (private: `Plain`, `Password`, `Number`).

## Key Functions/Methods
- `TextEntryDialog::new(frame, message, caption, default)` — pre-fills the `EDIT` with `default`.
- `TextEntryDialog::show_modal() -> Option<String>` — `Some(text)` on OK, `None` on cancel.
- `TextEntryDialog::set_message(&str)` / `set_value(&str)` / `message()` / `caption()` / `default_value()`.
- `PasswordEntryDialog::new(frame, message, caption)` — no default value (passwords aren't pre-typed).
- `PasswordEntryDialog::show_modal() -> Option<String>` — plain-text password, since the `EDIT` style is `ES_PASSWORD` only at the UI level.
- `PasswordEntryDialog::set_message(&str)` / `message()` / `caption()`.
- `NumberEntryDialog::new(frame, message, caption, initial)` — pre-fills with the string form of `initial`.
- `NumberEntryDialog::set_min(min) / set_max(max)` — out-of-range values cause `show_modal` to return `None`.
- `NumberEntryDialog::show_modal() -> Option<i64>` — parses the `EDIT` text with `str::parse::<i64>()`, applies the optional min/max, returns `None` on parse failure or out-of-range.
- `NumberEntryDialog::set_message(&str)` / `message()` / `caption()`.

## Quick start

```rust,no_run
use ru_wx::prelude::*;

// 1. Plain text input.
let dlg = TextEntryDialog::new(&frame, "Your name:", "Login", "Alice");
let answer: Option<String> = dlg.show_modal();
match answer {
    Some(name) => println!("hello, {name}"),
    None      => println!("cancelled"),
}

// 2. Password input. Returns the plain-text password (the EDIT uses ES_PASSWORD
//    only at the UI level; the value is decrypted to a String internally).
let pw = PasswordEntryDialog::new(&frame, "Password:", "Auth");
let pw_value: Option<String> = pw.show_modal();

// 3. Number input with optional bounds.
let n = NumberEntryDialog::new(&frame, "Age:", "Profile", 30);
n.set_min(0);
n.set_max(120);
let age: Option<i64> = n.show_modal();
if let Some(age) = age {
    if !(0..=120).contains(&age) {
        // out-of-range -> show_modal returned None automatically
    }
}
```

`show_modal` blocks until the user clicks OK or Cancel (or presses Enter / Escape in the EDIT, which is wired to OK / Cancel). Default focus is on the EDIT, so the user can start typing immediately.

## Win32 Notes
- Window class `RuWxTextEntryDialogClass` is registered once via `RegisterClassExW`.
- `EDIT` style is built as `WS_CHILD | WS_VISIBLE | WS_TABSTOP | WS_BORDER | ES_AUTOHSCROLL` plus `ES_PASSWORD` for passwords or `ES_NUMBER` for numeric input.
- `WS_EX_CLIENTEDGE` on the `EDIT` gives it a sunken native look.
- `IDOK` (1) and `IDCANCEL` (2) are the standard Win32 button ids, recognised in `entry_wnd_proc`'s `WM_COMMAND` branch.
- `WM_KEYDOWN` for `VK_RETURN` / `VK_ESCAPE` while focus is on the `EDIT` is treated as OK / Cancel respectively (so the user never has to leave the keyboard).
- `WM_CLOSE` / `WM_DESTROY` clear the result and break the modal loop. The `Rc` parked in `GWLP_USERDATA` is released in `WM_DESTROY`.
- Default focus is on the `EDIT` control, so the user can start typing immediately.
- Dialog dimensions: 360×150 px.

## See Also
- [`frame.rs`](./frame.md) — `frame.hwnd()` used as the parent.
- [`symbol_picker_dialog.rs`](./symbol_picker_dialog.md) — same skeleton with an extra `LISTBOX` on top.
- [`single_choice_dialog.rs`](./single_choice_dialog.md) — same skeleton with a `LISTBOX` instead of an `EDIT`.
- [`platform/win32.rs`](./platform/win32.md) — `next_control_id`, `to_wide`.
