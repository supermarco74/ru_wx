# progress_dialog.rs

Modeless progress dialog (`wxProgressDialog`) built from a custom `RuWxProgressDialogClass` top-level window containing a `STATIC` label, an `msctls_progress32` gauge, and an optional "Cancel" `BUTTON`.

## Purpose
Shows a determinate progress bar plus an operation message. The host application drives the dialog by calling [`ProgressDialog::update`] periodically; the call pushes a new value into the gauge, pushes a new message into the label, **pumps pending Win32 messages** so the Cancel button stays clickable, and returns `true` if the user has clicked Cancel since the last call.

This mirrors the documented `wxProgressDialog::Update` contract: the caller is expected to yield back to the dialog by calling `update`, which is the only safe way to combine a long-running main-thread operation with a responsive UI.

## Key Types
- `ProgressDialog` — `Clone`, holds `Rc<RefCell<ProgressDialogInner>>`. The inner stores `hwnd`, `hwnd_label`, `hwnd_gauge`, `hwnd_cancel`, `cancelled: bool`, `closed: bool`, `range: i32`, `value: i32`, `title`, `message`, `has_cancel: bool`.

## Key Functions/Methods
- `ProgressDialog::new(title, message, range)` — build a dialog **without** a Cancel button.
- `ProgressDialog::with_cancel_button(title, message, range)` — same, with a Cancel button.
- `ProgressDialog::show(&self)` — show the dialog window (`SW_SHOW` + `UpdateWindow`).
- `ProgressDialog::close(&mut self)` — destroy the dialog window.
- `ProgressDialog::update(value, message) -> bool` — update both value (clamped to `[0, range]`) and label, then pump and return `is_cancelled()`.
- `ProgressDialog::update_value(value) -> bool` — update the gauge only; leave the label.
- `ProgressDialog::update_message(message) -> bool` — update the label only; leave the value.
- `ProgressDialog::is_cancelled(&self) -> bool`
- `ProgressDialog::is_closed(&self) -> bool`
- `ProgressDialog::title() / message() / range() / value() / has_cancel_button()` — read-only accessors.

## Win32 Notes
- Window class `RuWxProgressDialogClass` is registered once via `RegisterClassExW`. `WNDCLASSEXW.hbrBackground` is `GetStockObject(0)` (the `NULL_BRUSH`) so the parent already-painted dialog frame shows through; in practice the title bar and frame are drawn by the system because the window uses `WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU` with `WS_EX_DLGMODALFRAME`.
- Default gauge: `PBM_SETRANGE32` (0x0406) + `PBM_SETPOS` (0x0402) + `PBM_GETPOS` (0x0408).
- The Cancel button uses `IDCANCEL = 2` as the menu id, so `WM_COMMAND` arriving in the `WndProc` can be matched by id alone.
- The WndProc parks the `Rc<RefCell<Inner>>` in `GWLP_USERDATA` via `SetWindowLongPtrW`, then re-fetches it on every `WM_COMMAND` to flip `cancelled = true` and on `WM_DESTROY` to release the raw pointer.
- `WM_CLOSE` is forwarded to `DestroyWindow`, which triggers `WM_DESTROY` and synchronously clears the `Rc`.
- `pump` uses `PeekMessageW` with `PM_REMOVE` in a tight loop so queued `WM_PAINT` / `WM_COMMAND` / `WM_MOUSEMOVE` from the Cancel button are dispatched before `update` returns.
- `Drop` calls `DestroyWindow` if the dialog hasn't been closed manually, so the `Rc` is always released.
- Dialog dimensions: 440×100 px without Cancel, 440×150 px with Cancel.

## Quick start

```rust
use ru_wx::prelude::*;

// frame is the owning Frame.
let pd = ProgressDialog::with_cancel_button("Working", "Loading…", 100);
pd.show();

// Drive a long task; update() pumps pending Win32 messages
// so the Cancel button stays clickable.
for i in 0..=100 {
    if pd.update(i, &format!("Step {i} / 100")) {
        // user pressed Cancel
        break;
    }
    // do_work_for_step(i);
}

pd.close();
```

`update(value, message)` is the only safe way to combine a long-running
main-thread operation with a responsive UI: it pushes a new value, pushes
a new label, pumps messages, and returns `true` if Cancel was clicked
since the last call. `close()` is also called automatically on `Drop`.

## See Also
- [`frame.rs`](./frame.md) — host frame pattern.
- [`widget.rs`](./widget.md) — `Window` trait, which `ProgressDialog` implements (so it can act as a parent if needed).
- [`platform/win32.rs`](./platform/win32.md) — `next_control_id`, `to_wide`.
- `examples/showcase_all.rs` — windowed smoke test for the progress dialog.
