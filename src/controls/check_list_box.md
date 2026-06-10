# check_list_box.rs

List with per-item check-boxes (`wxCheckListBox`). On Windows there is no `LBS_CHECKBOXES` style, so the module uses a plain `LISTBOX` and keeps a parallel `Vec<bool>` of checked states in the inner struct.

## Purpose
Click an item → the user code is invoked with `(index, new_checked)`. The struct auto-toggles the stored check state, then deselects the row (`LB_SETCURSEL(usize::MAX)`) so the highlight does not appear "stuck" after a click.

## Key Types
- `CheckListBox` — `Clone`, wraps `Rc<RefCell<CheckListBoxInner>>`. `CheckListBoxInner` holds `hwnd`, `id`, `rect`, `items: Vec<String>`, `checked: Vec<bool>`, `enabled`, `visible`.

## Key Functions/Methods
- `CheckListBox::new<W: Window>(parent)` — 200×200 px listbox.
- `CheckListBox::append(&self, item)` — appends unchecked item. `insert(index, item)` (clamps index), `remove(index)`, `clear`.
- `CheckListBox::get_count(&self) -> usize` — from the in-memory `items` vector (no FFI call).
- `CheckListBox::get_selection(&self) -> Option<usize>` / `get_selections(&self) -> Vec<usize>` — `LB_GETCURSEL` / `LB_GETSELCOUNT` + `LB_GETSELITEMS`.
- `CheckListBox::set_selection(&self, index)` — `LB_SETCURSEL`.
- `CheckListBox::get_string(&self, index) -> Option<String>` — from the in-memory `items` vector (cheap, no FFI).
- `CheckListBox::check(&self, index, checked: bool)` / `is_checked(&self, index) -> bool` / `get_checked_items(&self) -> Vec<bool>`.
- `CheckListBox::on_check_toggle<F: FnMut(usize, bool)>(&self, frame, cb)` — installs the `LBN_SELCHANGE` handler that performs the auto-toggle and deselect.
- `CheckListBox::id(&self) -> u16`, `CheckListBox::as_widget_ref(&self) -> WidgetRef`.

## Win32 Notes
- Class `LISTBOX`, style `LBS_NOTIFY = 0x0001` plus the standard `WS_CHILD | WS_VISIBLE | WS_BORDER | WS_VSCROLL`.
- The visible control is just a normal listbox; the check-box glyph is **not** drawn by Windows. The struct maintains the per-item state itself.
- `LB_SETCURSEL(usize::MAX)` clears the current selection (Win32 treats `-1`/`UINT_MAX` as "no selection").
- `on_check_toggle` clones the `Rc<RefCell<inner>>` into the closure so the handler can mutate state without holding a borrow across the callback invocation.

## Quick start

```rust
use ru_wx::prelude::*;

// frame is the owning Frame.
let clb = CheckListBox::new(&frame);
clb.append("Read the docs");
clb.append("Write tests");
clb.append("Ship it");

// Pre-check an item.
clb.check(0, true);

let summary = StaticText::new(&frame, "Checked: []");
let summary_for_cb = summary.clone();
clb.on_check_toggle(&frame, move |idx, checked| {
    // Re-render the list of checked indices:
    let mut checked_idx = Vec::new();
    for i in 0..clb.get_count() {
        if clb.is_checked(i) { checked_idx.push(i); }
    }
    summary_for_cb.set_label(&format!("Checked: {checked_idx:?}"));
});
```

On click, the control auto-toggles the stored check state and clears
its own selection highlight, so the user never sees a "stuck" row.

## See Also
- [`list_box.rs`](list_box.md) — non-checking variant. Same `LB_*` constants.
- [`checkbox.rs`](checkbox.md) — single independent check-box primitive.
- [`frame.rs`](../window/frame.md) — `register_command_handler` used by `on_check_toggle`.
- [`widget.rs`](../core/widget.md) — `Widget` trait.
