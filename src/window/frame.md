# frame.rs

The top-level window (`wxFrame` analog). Every `ru_wx` GUI starts with a `Frame`: the user constructs one with `Frame::builder()`, attaches widgets / menu bar / sizer / handlers, and finally calls `frame.show()` to enter the Win32 message loop.

## Purpose
The single top-level window of an application. It owns the dispatch tables for `WM_COMMAND`, `WM_NOTIFY`, `WM_HSCROLL`/`WM_VSCROLL`, `WM_PAINT`, accelerator keys, tray-icon messages, shell-level file drops, and OLE COM drops. It also owns the `BoxSizer` that lays out its children, the menu bar, the background colour, and the resize / close / drop callback slots.

## Key Types
- `Frame` — public struct, wraps `Rc<RefCell<FrameData>>`. Cloning the `Rc` is the standard way to share the frame with a widget callback.
- `FrameData` (pub(crate)) — internal state. Fields: `hwnd`, `widgets`, `command_handlers`, `notify_handlers`, `disp_info_handlers`, `dtn_handlers`, `tray_message_handlers`, `scroll_handlers`, `paint_handlers`, `accelerators`, `menu_bar`, `sizer`, `background_colour`, `on_resize`, `on_close`, `drop_files_handler`, `ole_drop_target`.
- `FrameBuilder` — `Frame::builder()` factory. Defaults: title `"ru_wx Window"`, size `800×600`, position `CW_USEDEFAULT`.

## Constructor / Lifecycle
- `Frame::builder() -> FrameBuilder` — entry point.
- `FrameBuilder::with_title(&str)`, `with_size(w, h)`, `with_position(x, y)`, `build()` (Windows-only; uses `CreateWindowExW` + `WS_OVERLAPPEDWINDOW`).
- `frame.show()` — enters the Win32 message loop. Stays until `WM_QUIT`. Builds an `HACCEL` from the registered accelerators, calls `ShowWindow(SW_SHOW)`, then `GetMessageW` / `TranslateAcceleratorW` / `TranslateMessage` / `DispatchMessageW`.
- `frame.close()` — `DestroyWindow` from user code; the wndproc also handles `WM_CLOSE` via the OS.

## Public Methods — High-level Layout
- `hwnd(&self) -> HWND` / `dpi(&self) -> Dpi` / `scale_factor(&self) -> f32`.
- `set_sizer(&self, sizer: BoxSizer)` — installs a sizer; re-layouts on resize.
- `set_menu_bar(&self, menubar: MenuBar)` — calls `SetMenu` + `DrawMenuBar`. Keeps an owned copy so accelerator mutators can refresh the menu's visible shortcut labels.
- `set_background_colour(&self, colour: Colour)` — `InvalidateRect(hwnd, null, 1)`.
- `on_resize<F: FnMut(u32, u32)>(&self, f)` — registers a resize callback. Multiple callbacks allowed; invoked in registration order on every `WM_SIZE`.
- `on_close<F: FnMut()>(&self, f)`.
- `set_title(&self, title: &str)`, `set_size(&self, w: u32, h: u32)`.

## Public Methods — Handlers
- `add_widget(&self, widget: WidgetRef)`.
- `register_command_handler(&self, id: u16, handler: Box<dyn FnMut()>)` — `WM_COMMAND` dispatch.
- `register_notify_handler(&self, id: u16, handler: Box<dyn FnMut(u32)>)` — `WM_NOTIFY` dispatch with the NMHDR `code` field as the callback argument.
- `register_disp_info_handler(&self, id: u16, handler: Box<dyn FnMut(isize)>)` — `LVN_GETDISPINFOW` for `LVS_OWNERDATA` virtual lists. The handler receives the full `lparam` (a `*mut NMLVDISPINFOW`) so it can both read the request and write the response.
- `register_dtn_handler(&self, id: u16, handler: Box<dyn FnMut(isize)>)` — `DTN_DATETIMECHANGE` for `DatePickerCtrl`. Receives `lparam` (a `*mut NMDATETIMECHANGE`).
- `register_tray_message_handler(&self, msg: u32, handler: Box<dyn FnMut(u32)>)` and `unregister_tray_message_handler(&self, msg: u32)` — used by `IconTray` for `NIN_*` notifications.
- `register_scroll_handler(&self, hwnd: HWND, handler: F)` where `F: FnMut(u16, i32)` and `unregister_scroll_handler` — `WM_HSCROLL` / `WM_VSCROLL` from child `SB_CTL` scroll bars. `code` is the `SB_*` request; `pos` is the thumb position for `SB_THUMBPOSITION` / `SB_THUMBTRACK`, `0` otherwise.
- `register_paint_handler(&self, handler: F)` where `F: FnMut(isize)`. The `isize` is the `HDC` from `BeginPaint`, suitable for direct GDI use or for the `PaintDC` wrapper.

## Public Methods — Accelerators
- `register_accelerator(&self, accel: Accelerator, command_id: u16)`. Duplicates allowed; first match wins in the Win32 `HACCEL` lookup.
- `accelerators(&self) -> Vec<(Accelerator, u16)>` — clone of the registration list.
- `unregister_accelerator(&self, accel: Accelerator) -> bool` — removes the **first** matching entry; returns `true` on success. Also clears the menu item's shortcut if a menubar is attached.
- `clear_accelerators(&self)` — empties the list; clears all menu items' shortcuts.
- `replace_accelerator(&self, old: Accelerator, new: Accelerator, command_id: u16) -> bool` — atomic rebind in the same slot. Menu item's shortcut is updated to `new`.

## Public Methods — Drag and Drop
- `set_drop_files_callback<F: FnMut(DroppedFiles)>(&self, f)` — Shell-level (`WM_DROPFILES`). One-shot handler: a second call replaces the first. The frame always calls `DragAcceptFiles(hwnd, 1)` at build time, so the dispatch is wired even if the user registers the callback after `build()`.
- `set_ole_drop_callback<F>(&self, f) -> Result<(), OleDropError>` — OLE COM (`IDropTarget`). Returns `OleDropError::RegisterFailed` if `RegisterDragDrop` fails. The OLE and Shell paths coexist; both can be registered on the same frame.

## Quick start

```rust,no_run
use ru_wx::prelude::*;

// 1) Build the frame. The builder defaults to "ru_wx Window",
//    800×600, CW_USEDEFAULT position.
let frame = Frame::builder()
    .with_title("My app")
    .with_size(800, 600)
    .build();

// 2) Populate the frame: sizer, children, menu bar, handlers.
//    The sizer owns the layout — you give it widgets and the
//    frame will lay them out on every WM_SIZE.
let sizer = BoxSizer::builder(Orientation::Vertical).build();
let button = Button::new(&frame, "Click me");
sizer.add(button.as_widget_ref(), 0, SizerFlag::Expand | SizerFlag::All, 8);
frame.set_sizer(sizer);

// 3) Hook up a click handler. register_command_handler dispatches
//    WM_COMMAND from any child control — buttons, menu items,
//    accelerators — keyed by the control's Win32 id.
let button_id = button.id();
frame.register_command_handler(button_id, Box::new(move || {
    println!("clicked!");
}));

// 4) Register an accelerator. The shortcut fires the same
//    command handler even when the owning menu is hidden.
frame.register_accelerator(
    Accelerator::new(Modifier::Ctrl, Key::S),
    button_id,
);

// 5) React to lifecycle events.
frame.on_resize(|w, h| println!("resized: {w}x{h}"));
frame.on_close(|| println!("closing"));

// 6) Enter the Win32 message loop. show() blocks until WM_QUIT.
frame.show();
```

Responding to user input beyond `WM_COMMAND` uses the
**handler-registration** family of methods, one per Win32
notification source:

```rust,no_run
use ru_wx::prelude::*;
// WM_NOTIFY from a child that uses notification messages
// (Tab, TreeCtrl, ListCtrl): the callback receives the NMHDR
// `code` field (e.g. TCN_SELCHANGE, TVN_SELCHANGED).
frame.register_notify_handler(tab_id, Box::new(|code| {
    if code == 0x0001 /* TCN_SELCHANGE */ { /* ... */ }
}));

// WM_PAINT: the callback gets the HDC cast to isize; use
// dc::PaintDC inside the closure for a typed wrapper.
frame.register_paint_handler(|hdc_isize| {
    let mut dc = unsafe { PaintDC::from_raw(hdc_isize) };
    dc.draw_line(0, 0, 200, 200);
});

// Shell-level file drops (DragAcceptFiles path).
frame.set_drop_files_callback(|files: DroppedFiles| {
    for path in files.paths() { println!("{}", path.display()); }
});

// OLE COM drops (RegisterDragDrop path) — both can coexist
// with set_drop_files_callback.
let _ = frame.set_ole_drop_callback(|_data: OleDroppedData, _pos| {
    // handle in-memory / virtual file drops
});
```

Mutating accelerators **after** `frame.show()` has started the
message loop is a no-op for the running session: the `HACCEL`
table is built once at loop entry. The intended pattern is to
register all accelerators during the construction phase, before
calling `show()`.

## Win32 Notes
- Window class: `"RuWxFrameClass"`, registered with `CS_HREDRAW | CS_VREDRAW` and the standard `IDC_ARROW` cursor.
- Default styles: `WS_OVERLAPPEDWINDOW`, `IDI_APPLICATION` for the class icon.
- **Re-entrancy-safe WndProc pattern**: every dispatch in `frame_wnd_proc` follows the same recipe:
  1. `GetWindowLongPtrW(hwnd, GWLP_USERDATA)` retrieves the raw `Rc<RefCell<FrameData>>` pointer stored at build time.
  2. `Rc::from_raw(ptr)` (NOT `Rc::increment_strong_count` + `Rc::from_raw`) reconstructs the strong `Rc`. `from_raw` already increments the refcount, and the matching `drop` at the end of the arm releases it, so the refcount round-trips to its pre-dispatch value. Calling `increment_strong_count` first would leak one strong reference on every dispatch.
  3. The handler / sizer / callback list is `take`n out of the `RefCell` so the borrow is released before any user code runs.
  4. The user callback runs; it may re-enter the frame without panicking.
  5. The list is put back; the `Rc` is dropped.
- The `WM_NOTIFY` arm dispatches to one of three maps based on the `NMHDR.code`:
  - `LVN_GETDISPINFOW` → `disp_info_handlers` (virtual-list disp-info)
  - `DTN_DATETIMECHANGE` → `dtn_handlers` (date-picker value change)
  - Everything else → `notify_handlers` (Tab `TCN_SELCHANGE`, ListCtrl `LVN_ITEMCHANGED`, etc.)
- `WM_PAINT` calls `BeginPaint` / `EndPaint` around the user's callback so the `HDC` is valid for GDI drawing.
- `WM_DROPFILES` extracts paths via `drop_target::extract_paths_from_hdrop`, calls the registered handler with a `DroppedFiles` value, then `DragFinish` to release the Shell handle.
- `WM_APP..=0xBFFF` (custom messages) is the range used by `IconTray` for shell notification area callbacks; the wndproc dispatches these to `tray_message_handlers`.
- The `HACCEL` accelerator table is built **once** at the start of `frame.show()` and destroyed when the message loop exits. Mutations to the accelerator list after the loop has started do **not** take effect on the running session.
- `do_layout` takes the sizer **out** of `FrameData` before calling `sizer.layout()` so the synchronous `WM_SIZE` / `WM_ERASEBKGND` re-emitted by `MoveWindow` cannot re-enter the frame's `RefCell` and panic.
- The test constructor `Frame::for_testing()` (cfg(test)) builds a `Frame` with a `null` `HWND` so the platform-agnostic surface (accelerator registration, command-handler dispatch table, sizer storage, OLE registration contract on non-Windows) can be exercised without spinning up a real Win32 message pump.

## Tests (subset; see `mod tests` in frame.rs)
- `for_testing_starts_with_empty_state` — initial `FrameData` is empty.
- `register_accelerator_preserves_order` / `register_accelerator_accepts_duplicates` / `accelerators_clone_is_isolated`.
- `unregister_accelerator_*`, `clear_accelerators_*`, `replace_accelerator_*`, `rebind_three_step_workflow` — accelerator mutation surface.
- `register_command_handler_*` / `register_notify_handler_appears_in_map` / `unregister_tray_message_handler_removes_entry`.
- `set_sizer_stores_and_can_be_replaced`.
- `dpi_falls_back_to_system_dpi_for_null_hwnd` / `scale_factor_matches_dpi_for_null_hwnd` (Windows-only).
- Menu-bar integration: `set_menu_bar_stores_the_menubar_in_frame_data`, `set_menu_bar_replaces_a_previous_menubar`, `replace_accelerator_refreshes_menu_label`, `unregister_accelerator_clears_menu_label`, `clear_accelerators_clears_all_menu_labels`, plus three "without_menubar" safety tests.
- Drop-files: `for_testing_starts_without_drop_files_handler`, `set_drop_files_callback_stores_handler` / `_replaces_previous` / `_keeps_handler_alive_across_borrows` / `_accepts_capturing_closure`.
- Disp-info: `register_disp_info_handler_stores_entry` / `_replaces_previous` / `signature_*` / `_accepts_capturing_closure` / `disp_info_and_notify_maps_are_independent`.
- DTN: parallel coverage for `dtn_handlers`.
- OLE: `set_ole_drop_callback_registers_or_fails_on_null_hwnd` (platform-split), `_replaces_previous` (non-Windows), `signature_set_ole_drop_callback`, `_accepts_capturing_closure` (non-Windows), `ole_drop_target_and_drop_files_handler_are_independent`, `ole_and_shell_drops_coexist`.

## See Also
- [`panel.rs`](panel.md) — typical immediate child of a `Frame`
- [`top_level_window.rs`](top_level_window.md) — composition wrapper adding iconize / maximize / full-screen
- [`menu.rs`](menu.md) — `MenuBar` passed to `set_menu_bar`
- [`accelerator.rs`](../core/accelerator.md) — `Accelerator` type used in `register_accelerator`
- [`sizer.rs`](../containers/sizer.md) — `BoxSizer` passed to `set_sizer`
- [`drop_target.rs`](../dnd/drop_target.md) — `DroppedFiles` and Shell-level dispatch helpers
- [`ole_dnd.rs`](../dnd/ole_dnd.md) — `OleDropTarget`, `OleDroppedData`, `OleDropError`, `OleDropPosition`
- [`icon_tray.rs`](../chrome/icon_tray.md) — uses `tray_message_handlers` for `NIN_*` notifications
- [`date_picker_ctrl.rs`](../controls/date_picker_ctrl.md) — uses `dtn_handlers` for `DTN_DATETIMECHANGE`
- [`list_ctrl.rs`](../controls/list_ctrl.md) — uses `disp_info_handlers` for `LVS_OWNERDATA`
- [`status_bar.rs`](../chrome/status_bar.md) — uses `add_resize_handler` to reapply field widths on `WM_SIZE`
