# list_box.rs

Single- or multi-selection list-box (`wxListBox`) on Windows — `LISTBOX` common control.

## Purpose
Always-visible scrollable list of strings, with optional extended multi-selection. `on_selection_change` and `on_double_click` both rely on `WM_COMMAND` notification codes (`LBN_SELCHANGE=1`, `LBN_DBLCLK=2`).

## Key Types
- `ListBox` — `Clone`, wraps `Rc<RefCell<ListBoxInner>>`. `ListBoxInner` holds `hwnd`, `id`, `rect`, `multi_select: bool`, `enabled`, `visible`.

## Key Functions/Methods
- `ListBox::new<W: Window>(parent)` — single-selection list, style `LBS_NOTIFY` (no `LBS_EXTENDEDSEL`).
- `ListBox::multi_select<W: Window>(parent)` — extended multi-selection list, adds `LBS_EXTENDEDSEL = 0x0800`.
- `ListBox::append` / `ListBox::insert(index, item)` / `ListBox::remove(index)` / `ListBox::clear` — `LB_ADDSTRING` (0x0180), `LB_INSERTSTRING` (0x0181), `LB_DELETESTRING` (0x0182), `LB_RESETCONTENT` (0x0184).
- `ListBox::get_count(&self) -> usize` — `LB_GETCOUNT` (0x018B).
- `ListBox::get_selection(&self) -> Option<usize>` — `LB_GETCURSEL` (0x0188); `None` on `LB_ERR` or if `multi_select`.
- `ListBox::get_selections(&self) -> Vec<usize>` — multi-select only: `LB_GETSELCOUNT` (0x0190) + `LB_GETSELITEMS` (0x0191) into a `Vec<u32>`.
- `ListBox::set_selection(&self, index)` — `LB_SETCURSEL` (0x0186).
- `ListBox::get_string(&self, index) -> Option<String>` — `LB_GETTEXTLEN` (0x018A) + `LB_GETTEXT` (0x0189).
- `ListBox::on_selection_change<F>(&self, frame, cb)` — handler for `LBN_SELCHANGE`.
- `ListBox::on_double_click<F>(&self, frame, cb)` — handler for `LBN_DBLCLK`.
- `ListBox::id(&self) -> u16`, `ListBox::as_widget_ref(&self) -> WidgetRef`.

## Win32 Notes
- Class `LISTBOX`. Standard window styles `WS_CHILD | WS_VISIBLE | WS_BORDER | WS_VSCROLL | LBS_NOTIFY`.
- `LBS_NOTIFY = 1` is mandatory: it tells the control to send `LBN_SELCHANGE` / `LBN_DBLCLK` notifications.
- `LBS_EXTENDEDSEL = 0x0800` enables Shift+Click and Ctrl+Click range selection.
- `LB_GETSELITEMS` writes selected indices into a caller-supplied `u32` buffer; the returned count can be less than the buffer size requested.
- Known limitation: `on_selection_change` and `on_double_click` both register on the **same control id**, so the `Frame::command_handlers` HashMap will only keep the last-registered one. To support both simultaneously, the WndProc would need to dispatch on `(id, notification_code)` rather than just `id`.
- Non-Windows stub: `get_count` returns 0, `get_selection` / `get_selections` / `get_string` return empty.

## Quick start

```rust
use ru_wx::prelude::*;

// frame is the owning Frame.
let lb = ListBox::new(&frame);
lb.append("alpha");
lb.append("beta");
lb.append("gamma");

// Multi-select variant (Shift+Click and Ctrl+Click range selection).
let multi = ListBox::multi_select(&frame);
multi.append("one");
multi.append("two");
multi.append("three");

let label = StaticText::new(&frame, "");
let multi_for_cb = multi.clone();
let label_for_cb = label.clone();
multi.on_selection_change(&frame, move || {
    let sels = multi_for_cb.get_selections();
    label_for_cb.set_label(&format!("Selected: {sels:?}"));
});

// React to double-click (e.g. open / execute).
let lb_for_dbl = lb.clone();
lb.on_double_click(&frame, move || {
    if let Some(i) = lb_for_dbl.get_selection() {
        if let Some(s) = /* lb_for_dbl.get_string(i) */ None { /* … */ }
    }
});
```

`LBS_NOTIFY` is mandatory for the selection / double-click events to
fire. `LB_GETSELITEMS` is only meaningful on `multi_select` lists; on
single-selection lists, use `get_selection()` instead.

## See Also
- [`check_list_box.rs`](check_list_box.md) — list with per-item check-boxes.
- [`choice.rs`](choice.md), [`combo_box.rs`](combo_box.md) — drop-down variants.
- [`list_ctrl.rs`](list_ctrl.md) — column-based list view.
- [`frame.rs`](../window/frame.md) — `register_command_handler` used by both event hooks.
- [`widget.rs`](../core/widget.md) — `Widget` trait.
