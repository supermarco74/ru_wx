# spin_ctrl_double.rs

Floating-point spin control. Realised as a `msctls_updown32` up-down control paired with an `EDIT` buddy, with the value stored internally as `value * 10^digits` and clamped to the Win32 16-bit position range.

## Purpose
Mirrors `wxSpinCtrlDouble`. Use it to step a `f64` value by a configurable increment with a fixed number of decimal digits. Like [`crate::spin_ctrl::SpinCtrl`], the up-down arrows and the editable text buddy are exposed as a single unit.

## Key Types
- `SpinCtrlDouble` — `Clone`, holds `Rc<RefCell<SpinCtrlDoubleInner>>`. `SpinCtrlDoubleInner` stores `updown_hwnd: HWND`, `edit_hwnd: HWND`, `id: u16`, `rect`, `min`, `max`, `value`, `increment`, `digits`, `enabled`, `visible`.

## Key Functions/Methods
- `SpinCtrlDouble::new<W: Window>(parent, initial, min, max, increment, digits)` — creates the control; `initial` is clamped to `[min, max]`. `digits` is the number of decimal places shown.
- `SpinCtrlDouble::set_range(&self, min, max) / get_range() -> (f64, f64)` — query / set the value range.
- `SpinCtrlDouble::set_value(&self, value: f64) / get_value() -> f64` — query / set the current value (clamped to `[min, max]`).
- `SpinCtrlDouble::get_min() -> f64 / get_max() -> f64` — individual range bounds.
- `SpinCtrlDouble::set_increment(&self, increment: f64) / get_increment() -> f64` — step in user units (must be > 0).
- `SpinCtrlDouble::set_digits(&self, digits: u32) / get_digits() -> u32` — number of decimal places shown.
- `SpinCtrlDouble::on_value_change<F: FnMut(f64)>(&self, frame, cb)` — register a callback fired on every user change (gets the new value).
- `SpinCtrlDouble::id(&self) -> u16` — returns the control id used for `WM_COMMAND` dispatch.
- `SpinCtrlDouble::as_widget_ref(&self) -> WidgetRef` — for use with sizers.

## Win32 Notes
- `msctls_updown32` class with `UDS_ALIGNRIGHT` (`0x0004`), `UDS_SETBUDDYINT` (`0x0002`), `UDS_ARROWKEYS` (`0x0020`), `UDS_HOTTRACK` (`0x0008`), `UDS_NOTHOUSANDS` (`0x0010`).
- `UDM_SETBUDDY` (`0x0469`) makes the up-down control target the EDIT buddy.
- The native up-down only handles 16-bit positions, so the integer position is `(value - min) * 10^digits / step_int` (clamped to `i16::MIN..=i16::MAX`).
- After every position change the buddy text is overwritten with `format!("{:.*}", digits, value)` so the user always sees the formatted double, not the raw integer.
- `UDM_SETBASE` is set to 10 (decimal) — internal use only.
- All FFI calls wrapped in `// SAFETY:` comments documenting validated arguments.

## Quick start

```rust
use ru_wx::prelude::*;

// frame is the owning Frame.
// Step a f64 in 0.1 increments, 2 decimal places shown.
let sp = SpinCtrlDouble::new(&frame, 1.0, 0.0, 10.0, 0.1, 2);

let label = StaticText::new(&frame, "Value: 1.00");
let sp_for_cb = sp.clone();
let label_for_cb = label.clone();
sp.on_value_change(&frame, move |new_value| {
    label_for_cb.set_label(&format!("Value: {new_value:.2}"));
});

// Adjust the step at runtime.
sp.set_increment(0.25);
```

`digits` controls how many decimals are shown in the edit buddy; the
internal value is a `f64`. Up-down only supports 16-bit positions, so
the wire value is `(value - min) * 10^digits / step_int` and clamped
to `i16`.

## See Also
- [`spin_ctrl.rs`](spin_ctrl.md) — integer version of this control.
- [`spin_button.rs`](spin_button.md) — up-down arrows without a text buddy.
- [`frame.rs`](../window/frame.md) — `Frame::register_command_handler` used by `on_value_change`.
- [`widget.rs`](../core/widget.md) — `Widget` trait, `Window` trait, `WidgetRef`.
- [`platform/win32.rs`](../platform/win32.md) — `next_control_id`, `to_wide`.
