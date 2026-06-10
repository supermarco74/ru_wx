# dialog.rs

Modal and modeless dialog window (`wxDialog` analog). Hosts child widgets and a dispatch table; the modal variant runs its own message loop that disables the parent and blocks until `end_modal`.

## Purpose
A self-contained popup window with a title, a fixed initial size, and (optionally) a modal loop. Use for confirmation prompts, settings panels, file-info dialogs, and any "blocking interaction" surface. Buttons are typically added with `create_button`; the user clicks a button → `end_modal` is called → `show_modal` returns the result.

## Key Types
- `Dialog` — public struct. Fields: `inner: Rc<RefCell<DialogData>>`, `parent_hwnd: HWND`.
- `DialogData` (pub(crate)) — `hwnd`, `widgets`, `command_handlers`, `result`, `modal_running`.
- `DialogButtonInner` (private) — Win32 `HWND`, `_id`, `rect`, `visible`, `enabled`. Returned inside a `WidgetRef` from `create_button`.

## Key Methods
- `Dialog::new(parent: &Frame, title: &str, width: u32, height: u32) -> Self` — Creates a modeless dialog. Modal flow is opt-in via `show_modal`.
- `show_modal(&self) -> i32` — Enters a local `GetMessageW` loop. Disables the parent for the duration. Returns the integer code passed to `end_modal`. Uses `IsDialogMessageW` so Tab navigation and the default-button behaviour work.
- `end_modal(&self, result: i32)` — Breaks the modal loop. The control flow returns from `show_modal` with the supplied code.
- `close(&self)` — Closes the dialog without setting a result.
- `create_button(&self, label: &str) -> (u16, WidgetRef)` — Adds a `BUTTON` child with `WS_TABSTOP`. Returns the auto-assigned control id and a `WidgetRef` for sizer integration.

## Win32 Notes
- Window class: `"RuWxDialogClass"`, registered with `CS_HREDRAW | CS_VREDRAW`.
- Styles: `WS_POPUP | WS_CAPTION | WS_SYSMENU | DS_MODALFRAME` (`0x80`). `DS_MODALFRAME` is the standard dialog frame style — gives the thin dialog border and standard caption.
- `dialog_wnd_proc` (unsafe `extern "system"`) handles:
  - `WM_COMMAND` — dispatches to the registered command handler for the child id.
  - `WM_CLOSE` — sets `modal_running = false` so the `GetMessageW` loop exits; the close button (X) therefore does **not** call `end_modal` automatically — call it explicitly if you want a specific result code.
  - `WM_DESTROY` — cleans up the `Rc<RefCell<DialogData>>` stored in `GWLP_USERDATA`.
- The modal loop calls `IsDialogMessageW(msg.hwnd, &msg)` so the dialog gets first crack at keyboard navigation (Tab, mnemonic, Esc) before dispatch.
- The dialog does **not** call `Frame::show`; it owns its own short-lived message loop. When the loop exits, the dialog window is destroyed and the function returns.

## Quick start

```rust
use ru_wx::prelude::*;

// frame is the owning Frame.
let dlg = Dialog::new(&frame, "Confirm", 320, 140);

// Add an OK and a Cancel button; create_button returns (id, WidgetRef).
let (id_ok, _wr_ok) = dlg.create_button("OK");
let (id_cancel, _wr_cancel) = dlg.create_button("Cancel");

// Install handlers; clicking X calls close() with no result.
let dlg_for_ok     = dlg.clone();
let dlg_for_cancel = dlg.clone();
dlg.register_command_handler(id_ok, move || dlg_for_ok.end_modal(1));
dlg.register_command_handler(id_cancel, move || dlg_for_cancel.end_modal(0));

// show_modal blocks until end_modal() / close() is called.
let result = dlg.show_modal();
if result == 1 { /* user confirmed */ }
```

`Dialog` owns its own short-lived `GetMessageW` loop; the parent `Frame` is
disabled for the duration. For a one-call OK/Cancel prompt, prefer the
lighter [`message_box`](../dialogs/message_box.md) helper.

## See Also
- [`frame.rs`](frame.md) — parent window pattern; a `Dialog` is always created with a `&Frame` reference
- [`message_box.rs`](../dialogs/message_box.md) — lighter alternative for simple OK/Cancel prompts
- [`message_dialog.rs`](../dialogs/message_dialog.md) — class-based wrapper around `MessageBoxW`
- [`file_dialog.rs`](../dialogs/file_dialog.md) — specialised dialog for file picking
- [`button.rs`](../controls/button.md) — `create_button` returns a `WidgetRef`; the `Button` API applies to the returned widget
