# list_ctrl.rs

Multi-column / icon / virtual list-view widget backed by the Win32 `SysListView32` common control (the same one wxWidgets' `wxListCtrl` wraps).

## Purpose
- Implements a single `ListCtrl` widget: columns, rows, single/multi selection, optional virtual mode (millions of rows without per-row allocation).
- Mirrors `wxListCtrl`'s `Report` / `List` / `Icon` / `SmallIcon` view styles (Win32 LVS_REPORT / LVS_LIST / LVS_ICON / LVS_SMALLICON).
- Exposes both "physical" (insert/set) and "virtual" (LVS_OWNERDATA + on-demand callback) APIs.

## Key Types
- `ListItem<'a>` — safe wrapper over the raw `LVITEMW` pointer the ListView hands back in `LVN_GETDISPINFOW`. Methods: `index()`, `sub_item()`, `is_text_requested()`, `set_text(&str) -> Result<(), &'static str>`. Buffer cap is 1024 UTF-16 code units (the typical `cchTextMax` the control provides).
- `ListCtrlStyle` — enum: `Report`, `List`, `Icon`, `SmallIcon`. Maps to LVS_*.
- `ListCtrlInner` — holds `hwnd`, `id`, `rect`, `col_count`, `item_count` (cached so `get_item_count` round-trips on a `null` HWND), `enabled`, `visible`, `on_item_selected`, `last_selection` (debounce), `on_get_disp_info` (virtual-mode callback).
- `ListCtrl` — public handle. `Rc<RefCell<ListCtrlInner>>`.

## Key Functions/Methods
- `ListCtrl::new<W: Window>(parent, style) -> Self` — creates SysListView32; for `Report` view, also issues `LVM_SETEXTENDEDLISTVIEWSTYLE` with `LVS_EX_FULLROWSELECT`.
- `insert_column(&self, index, title, width_px)` — `LVM_INSERTCOLUMN` with `LVCF_TEXT | LVCF_WIDTH`. Bumps `col_count`.
- `insert_item(&self, index, text) -> usize` — `LVM_INSERTITEM`, returns the inserted row index.
- `set_item_text / get_item_text` — `LVM_SETITEMTEXT` / `LVM_GETITEMTEXT` (with growable buffer up to 64 KB).
- `delete_item / delete_all_items` — `LVM_DELETEITEM` / `LVM_DELETEALLITEMS`.
- `get_selected_item` → `Option<usize>` (uses `LVM_GETNEXTITEM` + `LVNI_SELECTED`).
- `select(index) / deselect(index) / clear_selection / is_selected(index) / get_selected_item_count / get_selected_items` — selection state API on top of `LVM_SETITEMSTATE` / `LVM_GETITEMSTATE` and `LVM_GETSELECTEDCOUNT`.
- `set_extended_style(style)` — `LVM_SETEXTENDEDLISTVIEWSTYLE` with mask=0 (set all bits).
- `set_item_count(count)` — opts the control into **virtual mode** (`LVS_OWNERDATA` toggled via `SetWindowLongPtrW(GWL_STYLE)`) and pushes the new row count with `LVM_SETITEMCOUNT` + `LVSICF_NOINVALIDATEALL` (skips full redraw).
- `on_item_selected(frame, F: FnMut(Option<usize>))` — registers a `WM_NOTIFY` handler on the frame filtered for `LVN_ITEMCHANGED`; dedupes consecutive duplicates via `last_selection`.
- `on_get_disp_info(frame, F: FnMut(&mut ListItem))` — registers a `WM_NOTIFY` handler for `LVN_GETDISPINFOW`; the callback receives a `&mut ListItem` whose `set_text` writes into the control-owned buffer.
- `id()`, `as_widget_ref`, plus the standard `Widget` impl (`native_handle`, `set_position`, `set_size`, `set_visible`, `set_enabled`, `rect`).

## Win32 Notes
- Class: `SysListView32` (requires `InitCommonControlsEx` with `ICC_LISTVIEW_CLASSES`, done in `app.rs`).
- Messages used: LVM_INSERTCOLUMN (0x101B), LVM_INSERTITEM (0x1007), LVM_SETITEMTEXT (0x102E), LVM_GETITEMTEXT (0x102D), LVM_GETITEMCOUNT (0x1004), LVM_DELETEITEM (0x1008), LVM_DELETEALLITEMS (0x1009), LVM_GETNEXTITEM (0x100C), LVM_SETITEMSTATE (0x102B), LVM_GETITEMSTATE (0x102C), LVM_GETSELECTEDCOUNT (0x1032), LVM_SETEXTENDEDLISTVIEWSTYLE (0x1036), LVM_SETITEMCOUNT (0x102F).
- Notification codes: `LVN_ITEMCHANGED` (0xFFFFFF9B), `LVN_GETDISPINFOW` (0xFFFFFF4F). The W (Unicode) variant is used; the A variant is not supported.
- State bits: `LVIS_FOCUSED = 0x0001`, `LVIS_SELECTED = 0x0002`; `LVS_EX_FULLROWSELECT = 0x20`; `LVS_OWNERDATA = 0x1000` (aliases `LVS_OWNERDRAWFIXED`).
- Structs defined locally: `LVCOLUMNW`, `LVITEMW`, `NMLVDISPINFOW` (all `#[repr(C)]`, fixed layout matching `<commctrl.h>`).
- `get_selected_items` walk bounded by `count+1` iterations with a no-progress guard, so a null/invalid HWND returns `vec![]` instead of spinning.
- `LVS_OWNERDATA` cannot be flipped via `LVM_SETEXTENDEDLISTVIEWSTYLE`; the code uses `SetWindowLongPtrW(hwnd, GWL_STYLE, …)` which is the only supported path.
- `lparam` for the `LVN_GETDISPINFOW` handler is reinterpreted as `*mut NMLVDISPINFOW` — see the safety comment in `on_get_disp_info` for the lifetime justification.

## Tests
- Constant-pinning tests (Windows-only): `lvm_constants_have_expected_values`, `lvis_constants_have_expected_values`, `lvn_getdispinfow_has_expected_value`, `lvs_ownerdata_has_expected_value`, `lvm_setitemcount_has_expected_value`, `lvsicf_flags_have_expected_values`.
- Signature-pinning tests (always available): `signature_select`, `signature_deselect`, `signature_clear_selection`, `signature_is_selected`, `signature_get_selected_item_count`, `signature_get_selected_items`, `signature_set_item_state`, `signature_get_item_state`, `signature_set_item_count`, `signature_on_get_disp_info`.
- Null-HWND safety tests (Windows-only, via `Frame::for_testing()`): every new v0.5.2 method is exercised on a `ListCtrl` with a `null` HWND and must not panic; `set_item_count` / `get_item_count` round-trip is verified even though `SendMessageW` on null is a no-op.
- `on_get_disp_info_registers_handler_on_frame` — confirms the handler is inserted into `frame.inner.disp_info_handlers` keyed by the control id.

## Quick start

```rust
use ru_wx::prelude::*;

// frame is the owning Frame.
let lc = ListCtrl::new(&frame, ListCtrlStyle::Report);
lc.insert_column(0, "Name", 200);
lc.insert_column(1, "Size", 80);

let r0 = lc.insert_item(0, "alpha.txt");
lc.set_item_text(r0, 1, "1024");

let r1 = lc.insert_item(1, "beta.txt");
lc.set_item_text(r1, 1, "2048");

// React to row clicks.
let label = StaticText::new(&frame, "");
let lc_for_cb = lc.clone();
let label_for_cb = label.clone();
lc.on_item_selected(&frame, move |sel| {
    if let Some(i) = sel {
        label_for_cb.set_label(&format!("Row {i}"));
    }
});

// Virtual mode: handle millions of rows without allocating them.
let virt = ListCtrl::new(&frame, ListCtrlStyle::Report);
virt.insert_column(0, "Index", 100);
virt.set_item_count(1_000_000);
virt.on_get_disp_info(&frame, move |item| {
    if item.sub_item() == 0 {
        let _ = item.set_text(&format!("Row {}", item.index()));
    }
});
```

`LVS_OWNERDATA` (virtual mode) can only be flipped via
`SetWindowLongPtrW(GWL_STYLE, …)` — the module handles that for you in
`set_item_count`. `on_get_disp_info` writes into the control's own
buffer (1024 UTF-16 code units), so a successful `set_text` returns
`Ok(())`.

## See Also
- [`frame.rs`](./frame.md) — provides `register_notify_handler` / `register_disp_info_handler` used by `on_item_selected` and `on_get_disp_info`.
- [`list_box.rs`](./list_box.md) — simpler LISTBOX primitive, single-column only.
- [`widget.rs`](./widget.md) — `Widget` trait implementation; `as_widget_ref` and sizer integration.
- [`lib.rs`](./lib.md) — `next_control_id()` allocator.
