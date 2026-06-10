# date_picker_dialog.rs

Modal date-picker dialog (`wxDatePickerDialog`).

## Purpose
Self-contained modal popup that lets the user pick a date on a `SysDateTimePick32` control. The dialog is implemented with the same skeleton as the other modal dialogs in this crate: a custom WndProc parked on `GWLP_USERDATA` via `Rc<RefCell<…>>`, plus a Win32 modal message loop. Unlike [`crate::date_picker_ctrl::DatePickerCtrl`] (which is an in-place child control you embed in a sizer), this dialog owns the picker, blocks the calling thread, and returns the picked `Date`.

## Key Types
- `DatePickerDialog` — owns a `SysDateTimePick32` control plus an OK and a Cancel button.
- `DateDialogFormat` — `enum { Short, Long }`. `Short` = the locale's short date format, `Long` = e.g. "Friday, June 5, 2026".

## Key Functions/Methods
- `DatePickerDialog::new(frame, message, caption, initial: Date)` — short format, no `DTS_SHOWNONE` (the user cannot clear the date).
- `DatePickerDialog::with_long_format(frame, message, caption, initial: Date)` — long format.
- `DatePickerDialog::with_allow_none(frame, message, caption)` — short format, `DTS_SHOWNONE` enabled.
- `DatePickerDialog::with_format_and_allow_none(frame, message, caption, initial, format, allow_none)` — fully custom constructor.
- `DatePickerDialog::show_modal() -> Option<Date>` — runs the modal loop. `Some(d)` on OK, `None` on cancel. With `DTS_SHOWNONE`, the user can also uncheck the date and click OK; the dialog returns `Some(d)` in that case if the user re-checked it, otherwise the dialog still returns `None` (consistent with the "no date set" path).
- `set_message`, `message`, `caption`, `initial_value`, `allows_none`, `format` — query / set the corresponding field.

## Win32 Notes
- Custom window class `RuWxDatePickerDialogClass` registered once (idempotent).
- `SysDateTimePick32` class with `DTS_LONGDATEFORMAT` (`0x0004`) for the long format, `DTS_SHOWNONE` (`0x0002`) for the optional checkbox.
- `DTM_GETSYSTEMTIME` (`0x1001`) / `DTM_SETSYSTEMTIME` (`0x1002`) to read / write the picked date.
- `GDT_VALID` (`0`), `GDT_NONE` (`1`) — flags returned by `DTM_GETSYSTEMTIME` to indicate whether the user has a date set.
- A local `SystemTime` struct (8 × `u16`) shadows the native one because `windows-sys 0.59` does not export it.
- The modal loop uses `GetMessageW` + `IsDialogMessageW` + `DispatchMessageW` and a `PeekMessageW` pump on the parent frame so the parent doesn't appear frozen.
- All FFI calls wrapped in `// SAFETY:` comments documenting validated arguments.

## Quick start

```rust
use ru_wx::prelude::*;

// frame is the owning Frame.
let today = Date::today();
let dlg = DatePickerDialog::new(
    &frame,
    "Pick a date",
    "Choose",
    today,
);

// show_modal blocks until the user clicks OK or Cancel.
if let Some(picked) = dlg.show_modal() {
    println!("User picked: {picked}");
}

// Long format ("Friday, June 5, 2026"):
let long_dlg = DatePickerDialog::with_long_format(
    &frame, "Pick a date", "Choose", today,
);

// Allow the user to clear the date (DTS_SHOWNONE):
let none_dlg = DatePickerDialog::with_allow_none(
    &frame, "Optional date", "Choose",
);
```

Unlike [`DatePickerCtrl`](../controls/date_picker_ctrl.md) (which is a child
control embedded in a sizer), `DatePickerDialog` owns the picker and
blocks the calling thread until dismissed.

## See Also
- [`date_picker_ctrl.rs`](../controls/date_picker_ctrl.md) — in-place child version of this control.
- [`text_entry_dialog.rs`](text_entry_dialog.md) — sibling modal dialog pattern (WndProc, modal loop, OK / Cancel dispatch).
- [`frame.rs`](../window/frame.md) — `Frame` parent.
