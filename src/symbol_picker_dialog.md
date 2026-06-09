# symbol_picker_dialog.rs

Symbol-picker modal dialog (`wxSymbolPickerDialog`) — the ru_wx analogue of "Insert Special Character".

## Purpose
Lets the user pick a single character or symbol from a list, with the option of typing one directly. The dialog contains a prompt `STATIC`, a `LISTBOX` populated with the supplied symbol list, a single-line `EDIT` control that mirrors the current selection, and OK / Cancel buttons. On OK it returns the selected symbol as a `String`; on cancel it returns `None`.

## Key Types
- `SymbolPickerDialog` — wraps `Rc<RefCell<SymbolDialogInner>>` plus `message` and `caption`.
- `SymbolDialogInner` — stores `hwnd`, `hwnd_label`, `hwnd_list`, `hwnd_edit`, `result: Option<String>`, `is_done: bool`.

## Key Functions/Methods
- `SymbolPickerDialog::new(frame, message, caption, symbols, initial)` — pass `usize::MAX` for `initial` to mean "no initial selection". The `symbols` slice is stored verbatim as listbox rows; rows can be a single Unicode codepoint, a short string, or a multi-character glyph.
- `SymbolPickerDialog::show_modal() -> Option<String>` — returns the chosen symbol or `None` on cancel.
- `SymbolPickerDialog::set_message(&str)` / `set_caption(&str)` — `SetWindowTextW` on the label / title.
- `SymbolPickerDialog::message() / caption()`.

## Win32 Notes
- Window class `RuWxSymbolPickerDialogClass` is registered once via `RegisterClassExW`.
- Listbox messages: `LB_ADDSTRING` (0x0180), `LB_SETCURSEL` (0x0186), `LB_GETCURSEL` (0x0188), `LB_GETTEXTLEN` (0x018A), `LB_GETTEXT` (0x0189).
- Listbox style: `WS_CHILD | WS_VISIBLE | WS_BORDER | WS_VSCROLL | LBS_NOTIFY`, extended `WS_EX_CLIENTEDGE`.
- Edit control style: `WS_CHILD | WS_VISIBLE | WS_BORDER | ES_AUTOHSCROLL | ES_CENTER`, extended `WS_EX_CLIENTEDGE`.
- On OK the WndProc reads the `EDIT` text first (so a user-typed character wins over the listbox selection); if the edit is empty, it falls back to `LB_GETCURSEL` + `LB_GETTEXTLEN` + `LB_GETTEXT`.
- `WM_KEYDOWN` for `VK_RETURN` / `VK_ESCAPE` is routed to OK / Cancel.
- `WM_CLOSE` / `WM_DESTROY` clear the result and break the modal loop. The `Rc` parked in `GWLP_USERDATA` is released in `WM_DESTROY`.
- Default focus is on the listbox.
- Dialog dimensions: 360×320 px.

## Quick start

```rust
use ru_wx::prelude::*;

// frame is the owning Frame.
let symbols = ["α", "β", "γ", "δ", "ε", "→", "←", "★"];
let dlg = SymbolPickerDialog::new(
    &frame,
    "Pick a symbol",
    "Insert Special Character",
    &symbols,
    usize::MAX,                 // no initial selection
);

if let Some(sym) = dlg.show_modal() {
    println!("Inserted: {sym}");
}
```

On OK, the dialog returns the *edit-field text first* (so a user-typed
character wins over the listbox selection); if the edit is empty, it
falls back to the listbox selection. `show_modal` returns `None` on
cancel. Like [`single_choice_dialog`](./single_choice_dialog.md) and
[`text_entry_dialog`](./text_entry_dialog.md), it shares the same
modal-loop skeleton.

## See Also
- [`frame.rs`](./frame.md) — `frame.hwnd()` used as the parent.
- [`single_choice_dialog.rs`](./single_choice_dialog.md) — same skeleton without the `EDIT` control.
- [`text_entry_dialog.rs`](./text_entry_dialog.md) — single-line edit only.
- [`platform/win32.rs`](./platform/win32.md) — `next_control_id`, `to_wide`.
