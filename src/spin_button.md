# spin_button.rs

Spin-button control mapped to Win32 `msctls_updown32` common control class. Unlike [`crate::spin_ctrl::SpinCtrl`], the spin button is *just* the up / down arrows — there is no associated text field. The current value is maintained in Rust; the control just emits a notification when the user clicks the arrows or presses the up / down keys.

## Purpose
Mirrors `wxSpinButton`. Use it when you want the user to step through a value but don't need the value to be displayed in an editable text field (e.g. the value is rendered by a separate widget, or you only need a delta).

## Key Types
- `SpinButton` — `Clone`, holds `Rc<RefCell<SpinButtonInner>>`. `SpinButtonInner` stores `hwnd: HWND`, `id: u16`, `rect`, `min`, `max`, `value`, `wrap`, `enabled`, `visible`.

## Key Functions/Methods
- `SpinButton::new<W: Window>(parent, min, max, initial)` — creates a non-wrapping spin button.
- `SpinButton::with_wrap<W: Window>(parent, min, max, initial, wrap: bool)` — `wrap = true` makes the value cycle at the extremes (`UDS_WRAP`).
- `SpinButton::set_range(&self, min, max) / get_range() -> (i32, i32)` — query / set the value range.
- `SpinButton::set_value(&self, value: i32) / get_value() -> i32` — query / set the current value (clamped to `[min, max]`).
- `SpinButton::get_min() -> i32 / get_max() -> i32` — individual range bounds.
- `SpinButton::on_value_change<F: FnMut()>(&self, frame, cb)` — register a callback fired on every user change.
- `SpinButton::id(&self) -> u16` — returns the control id used for `WM_COMMAND` dispatch.
- `SpinButton::as_widget_ref(&self) -> WidgetRef` — for use with sizers.

## Win32 Notes
- `msctls_updown32` class.
- `UDS_WRAP` (`0x0001`) — wrap-around at the extremes.
- `UDS_ARROWKEYS` (`0x0020`) — process up / down arrow keys.
- `UDS_HOTTRACK` (`0x0008`) — hot-track the arrows as the user drags.
- `UDS_NOTHOUSANDS` (`0x0010`) — no thousands separator in any internal text.
- `UDM_SETRANGE` (`0x0465`) — `wparam = 0`, `lparam = (max << 16) | min`. Win32 up-down only supports 16-bit range; `set_range` clamps the values.
- `UDM_SETPOS` (`0x0467`), `UDM_GETPOS` (`0x0468`) — read / write the current position.
- All FFI calls wrapped in `// SAFETY:` comments documenting validated arguments.

## Quick start

```rust
use ru_wx::prelude::*;

// frame is the owning Frame.
let sb = SpinButton::new(&frame, 0, 10, 0);  // min, max, initial

// Wrap-around variant (cycles at the extremes).
let wrap = SpinButton::with_wrap(&frame, 0, 5, 0, true);

let label = StaticText::new(&frame, "Step: 0");
let sb_for_cb = sb.clone();
let label_for_cb = label.clone();
sb.on_value_change(&frame, move || {
    label_for_cb.set_label(&format!("Step: {}", sb_for_cb.get_value()));
});
```

Unlike [`SpinCtrl`](./spin_ctrl.md), `SpinButton` has **no** text
buddy — the value is maintained in Rust. Use it when the value is
rendered elsewhere (e.g. by a custom `Dc` draw) or you only need a
delta. Win32 up-down only supports a 16-bit range, so
`set_range(min, max)` clamps the wire value.

## See Also
- [`spin_ctrl.rs`](./spin_ctrl.md) — `SpinCtrl` (up-down + EDIT buddy for integer values).
- [`spin_ctrl_double.rs`](./spin_ctrl_double.md) — `SpinCtrlDouble` (floating-point version).
- [`frame.rs`](./frame.md) — `Frame::register_command_handler` used by `on_value_change`.
- [`widget.rs`](./widget.md) — `Widget` trait, `Window` trait, `WidgetRef`.
- [`platform/win32.rs`](./platform/win32.md) — `next_control_id`, `to_wide`.
