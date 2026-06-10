# combo_box.rs

Editable combo-box (`wxComboBox`) on Windows — `COMBOBOX` with `CBS_DROPDOWN`. The companion [`choice.rs`](choice.md) module provides the read-only `CBS_DROPDOWNLIST` variant.

## Purpose
A drop-down list with an editable text field. Two constructors:
- `ComboBox::new` → `CBS_DROPDOWN = 0x0002` (user can type or pick).
- `ComboBox::choice` → `CBS_DROPDOWNLIST = 0x0003` (read-only, identical to `Choice`).

`editable: bool` is stored in the inner struct for future dispatch (currently the selection-change handler does not branch on it).

## Key Types
- `ComboBox` — `Clone`, wraps `Rc<RefCell<ComboBoxInner>>`. `ComboBoxInner` holds `hwnd`, `id`, `rect`, `editable`, `enabled`, `visible`.

## Key Functions/Methods
- `ComboBox::new<W: Window>(parent)` — editable combo.
- `ComboBox::choice<W: Window>(parent)` — read-only drop-down.
- `ComboBox::append` / `ComboBox::insert(index, item)` / `ComboBox::remove(index)` / `ComboBox::clear` — `CB_ADDSTRING` (0x0143), `CB_INSERTSTRING` (0x014A), `CB_DELETESTRING` (0x0144), `CB_RESETCONTENT` (0x014B).
- `ComboBox::get_count(&self) -> usize` — `CB_GETCOUNT` (0x0146); returns 0 on `CB_ERR`.
- `ComboBox::get_selection(&self) -> Option<usize>` — `CB_GETCURSEL` (0x0147); `None` on `CB_ERR`.
- `ComboBox::set_selection(&self, index)` — `CB_SETCURSEL` (0x014E).
- `ComboBox::get_value(&self) -> String` — `GetWindowTextLengthW` + `GetWindowTextW` on the combo's edit field. Only meaningful for `CBS_DROPDOWN`.
- `ComboBox::set_value(&self, text)` — `SetWindowTextW` on the edit field.
- `ComboBox::on_selection_change<F: FnMut() + 'static>(&self, frame, cb)` — registers selection-change handler.
- `ComboBox::id(&self) -> u16`, `ComboBox::as_widget_ref(&self) -> WidgetRef`.

## Win32 Notes
- Class `COMBOBOX`. Style differs by constructor (`CBS_DROPDOWN` vs `CBS_DROPDOWNLIST`).
- `set_size` / `set_position` pass `200` as the height to `MoveWindow`: for a combo this is the **drop-down list** height, not the visible control height. So the collapsed widget stays ~24 px tall regardless of sizer-assigned height.
- `get_value` uses `GetWindowTextW` on the combo HWND itself, which returns the edit-field text on `CBS_DROPDOWN`.
- `get_value` is unused for `CBS_DROPDOWNLIST` (always returns the currently selected item string from the read-only text field).
- `CB_ERR` is compared as `result == CB_ERR as isize` to avoid type-mismatch warnings on `isize` returns.
- `editable` field is `#[allow(dead_code)]` — reserved for future per-style behaviour.

## Quick start

```rust
use ru_wx::prelude::*;

// frame is the owning Frame.
let c = ComboBox::new(&frame);              // editable (CBS_DROPDOWN)
c.append("Apple");
c.append("Banana");
c.append("Cherry");
c.set_selection(1);

let label = StaticText::new(&frame, "");
let c_for_cb = c.clone();
let label_for_cb = label.clone();
c.on_selection_change(&frame, move || {
    if let Some(i) = c_for_cb.get_selection() {
        if let Some(text) = /* c_for_cb.get_string(i) */ None { /* … */ }
    }
    // For an editable combo, also fetch what the user typed:
    let _typed = c_for_cb.get_value();
    label_for_cb.set_label("changed");
});

// Read-only drop-down (identical to Choice):
let ro = ComboBox::choice(&frame);
```

`get_value()` is only meaningful on the editable variant — on
`CBS_DROPDOWNLIST` it just reads the currently selected item string.

## See Also
- [`choice.rs`](choice.md) — read-only-only variant.
- [`text_ctrl.rs`](text_ctrl.md) — single-line text input without a drop-down.
- [`list_box.rs`](list_box.md) — visible (non-drop-down) list.
- [`frame.rs`](../window/frame.md) — `register_command_handler`.
- [`widget.rs`](../core/widget.md) — `Widget` trait.
