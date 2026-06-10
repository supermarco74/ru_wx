# single_choice_dialog.rs

Single- and multi-choice modal dialogs (`wxSingleChoiceDialog`, `wxMultiChoiceDialog`).

## Purpose
Two related modal popups that show a prompt, a list of choices, and OK / Cancel buttons. Both share the same `RuWxChoiceDialogClass` window class and the same `ChoiceDialogInner` data; they differ only in the listbox style and result type.

| Dialog | Control | Result |
|---|---|---|
| `SingleChoiceDialog` | single-select `LISTBOX` | `Option<usize>` |
| `MultiChoiceDialog`  | `LISTBOX` with `LBS_EXTENDEDSEL` | `Option<Vec<usize>>` |

There is also a shared `ChoiceResult` enum with `Cancelled`, `Single(usize)`, and `Multi(Vec<usize>)` variants and a `is_cancelled(&self) -> bool` method. This is re-exported from the crate root.

## Key Types
- `SingleChoiceDialog` — wraps an `Rc<RefCell<ChoiceDialogInner>>` plus `message`, `caption`, `initial: Option<usize>`.
- `MultiChoiceDialog` — same skeleton, no `initial`.
- `ChoiceDialogInner` — stores `hwnd`, `hwnd_label`, `hwnd_list`, `result: ChoiceResult`, `is_done: bool`, `choices_count: usize`, `kind: ChoiceKind` (private: `Single` or `Multi`).
- `ChoiceResult` — public enum used by the result of `show_modal`.

## Key Functions/Methods
- `SingleChoiceDialog::new(frame, message, caption, choices, initial)` — pass `usize::MAX` for `initial` to mean "no initial selection".
- `SingleChoiceDialog::show_modal() -> Option<usize>` — returns the selected index or `None` on cancel.
- `SingleChoiceDialog::set_message(&str)` / `set_caption(&str)` — `SetWindowTextW` on the label / title.
- `SingleChoiceDialog::message() / caption()` — read the stored text.
- `MultiChoiceDialog::new(frame, message, caption, choices)` — no initial selection (Ctrl/Shift + Click to multi-select).
- `MultiChoiceDialog::show_modal() -> Option<Vec<usize>>` — returns the selected indices in selection order.
- `MultiChoiceDialog::set_message(&str)` / `set_caption(&str)` — same as above.
- `MultiChoiceDialog::message() / caption()`.

## Win32 Notes
- Window class `RuWxChoiceDialogClass` is registered once via `RegisterClassExW`.
- Listbox messages: `LB_ADDSTRING` (0x0180), `LB_SETCURSEL` (0x0186), `LB_GETCURSEL` (0x0188), `LB_GETSELCOUNT` (0x0190), `LB_GETSELITEMS` (0x0191).
- Listbox style: `WS_CHILD | WS_VISIBLE | WS_BORDER | WS_VSCROLL | LBS_NOTIFY`, plus `LBS_EXTENDEDSEL` (0x0800) for the multi variant.
- Default focus is on the listbox.
- `WM_COMMAND` is dispatched in `choice_wnd_proc`:
  - `IDOK` / `IDCANCEL` — stores the result (`Single(i)`, `Multi(v)` or `Cancelled`) and `is_done = true`, then `DestroyWindow`.
  - `LBN_DBLCLK` (notification code 2) — double-click is treated as OK.
  - `WM_KEYDOWN` for `VK_RETURN` / `VK_ESCAPE` — also routed to OK / Cancel.
- The `WndProc` parks the `Rc<RefCell<Inner>>` in `GWLP_USERDATA` and re-fetches it on every `WM_COMMAND` to mutate the result. `WM_DESTROY` releases the `Rc`.
- The modal loop (`run_choice_modal_loop`) uses `GetMessageW` + `IsDialogMessageW` + `DispatchMessageW`, polling `is_done` after each dispatch.
- Dialog dimensions: 360×280 px.

## Quick start

```rust
use ru_wx::prelude::*;

// frame is the owning Frame.
let choices = ["Small", "Medium", "Large"];

// --- Single-select ------------------------------------------------
let dlg = SingleChoiceDialog::new(
    &frame,
    "Pick a size:",
    "Size",
    &choices,
    1,                       // initial selection
);
if let Some(idx) = dlg.show_modal() {
    println!("Picked {} (index {idx})", choices[idx]);
}

// --- Multi-select (Ctrl/Shift + Click) ----------------------------
let multi = MultiChoiceDialog::new(
    &frame,
    "Pick toppings:",
    "Toppings",
    &["Cheese", "Mushroom", "Pepperoni", "Olive"],
);
if let Some(indices) = multi.show_modal() {
    println!("Picked {indices:?}");
}
```

Double-click is treated as OK. The shared `ChoiceResult` enum (with
`Cancelled`, `Single`, `Multi` variants) is also re-exported from the
crate root for callers that need a single uniform return type.

## See Also
- [`frame.rs`](../window/frame.md) — `frame.hwnd()` used as the parent.
- [`list_box.rs`](../controls/list_box.md) — non-modal `LISTBOX` widget for embedded UI.
- [`platform/win32.rs`](../platform/win32.md) — `next_control_id`, `to_wide`.
- [`symbol_picker_dialog.rs`](symbol_picker_dialog.md) — same skeleton with an extra `EDIT` control.
- [`text_entry_dialog.rs`](text_entry_dialog.md) — same skeleton with just an `EDIT` and no listbox.
