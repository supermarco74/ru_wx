# spin_ctrl.rs

Numeric up-down stepper paired with a read-only EDIT buddy, backed by the Win32 `msctls_updown32` common control.

## Purpose
- Implements a single `SpinCtrl` widget: a composite of an `msctls_updown32` (up-down arrows) and a flat `EDIT` field that the up-down writes into automatically.
- Mirrors `wxSpinCtrl`: clamped integer range, current value, change event.
- The composite is exposed as a single `Widget` (the EDIT's HWND is the sizer-anchor); moving the parent anchor moves both visually as one unit.

## Key Types
- `SpinCtrlInner` — holds `updown_hwnd`, `edit_hwnd`, `id`, `rect`, `min`, `max`, `value`, `enabled`, `visible`. Single allocation behind `Rc<RefCell<…>>`.
- `SpinCtrl` — public handle. Cheap to clone (`Rc`).
- `SpinCtrlValueChangeFn` — `Rc<dyn Fn(&SpinCtrl, i32)>` callback alias for `on_value_change`.

## Key Functions/Methods
- `SpinCtrl::new<W: Window>(parent, min, max, initial) -> Self` — creates the EDIT buddy (ES_AUTOHSCROLL | ES_NUMBER) and the up-down (UDS_ALIGNRIGHT | UDS_SETBUDDYINT | UDS_ARROWKEYS | UDS_HOTTRACK | UDS_NOTHOUSANDS), wires them with UDM_SETBUDDY, seeds UDM_SETBASE=10 and UDM_SETRANGE.
- `set_range(&self, min: i32, max: i32)` — re-issues UDM_SETRANGE.
- `set_value(&self, value: i32)` — UDM_SETPOS (clamped).
- `get_value(&self) -> i32` — UDM_GETPOS (HIWORD may be set to non-zero when value changed; masked out).
- `get_min/get_max/get_range` — cached in `Inner`, no Win32 call.
- `on_value_change(&self, f)` — registers the callback on the up-down's control id; fires on UDN_DELTAPOS / EN_CHANGE.
- `id`, `as_widget_ref`, plus inherited `Window`/`Widget` methods.

## Win32 Notes
- Class: `msctls_updown32` (requires `InitCommonControlsEx` from `comctl32` — done in `app.rs`).
- Messages used: UDM_SETBUDDY (0x0469), UDM_SETBASE (0x0465), UDM_SETRANGE (0x046D), UDM_SETPOS (0x046B), UDM_GETPOS (0x0468).
- UDM_SETRANGE `wparam` packs `(max << 16) | min` and only the low 16 bits of each are honored; this file intentionally caps the range to `i16::MIN..=i16::MAX` in the wire value (the cached i32 fields keep the logical range).
- UDM_GETPOS returns `((pos & 0xFFFF) | ((err & 1) << 16))` — file masks with `& 0xFFFF`.
- `Widget::native_handle` returns the EDIT HWND so a `Sizer` rows/columns the pair as one cell (the up-down is positioned automatically with UDS_ALIGNRIGHT inside the EDIT's client area).
- `Drop` calls `DestroyWindow` on the up-down first, then the EDIT.
- The EDIT is created with ES_NUMBER so non-digit paste is silently dropped by the OS.

## Quick start

```rust
use ru_wx::prelude::*;

// frame is the owning Frame.
let sp = SpinCtrl::new(&frame, 0, 100, 5);    // min, max, initial

let label = StaticText::new(&frame, "Value: 5");
let sp_for_cb = sp.clone();
let label_for_cb = label.clone();
sp.on_value_change(&frame, move || {
    let v = sp_for_cb.get_value();
    label_for_cb.set_label(&format!("Value: {v}"));
});

// Programmatic changes.
sp.set_range(0, 1000);
sp.set_value(250);
```

The control is a composite of an `msctls_updown32` and a flat
`ES_NUMBER`-style EDIT; the up-down writes into the EDIT
automatically (`UDS_SETBUDDYINT`). The Wire value is 16-bit (the
internal i32 keeps the logical range). The EDIT HWND is the
sizer-anchor, so the pair moves as one cell.

## See Also
- [`text_ctrl.rs`](text_ctrl.md) — same EDIT primitive, used standalone here as the buddy.
- [`button.rs`](./button.rs) — sibling pattern of a single Win32 common control wrapped in `*Inner`.
- [`lib.rs`](../lib.md) — `next_control_id()` allocator.
