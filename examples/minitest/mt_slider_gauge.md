# mt_slider_gauge.rs

Minitest for [`Slider`](file:///f:/code/ru_wx/ru_wx/src/slider.rs), [`Gauge`](file:///f:/code/ru_wx/ru_wx/src/gauge.rs) and [`SpinCtrl`](file:///f:/code/ru_wx/ru_wx/src/spin_ctrl.rs) — the three numeric input/display controls.

**Run:** `cargo run --example mt_slider_gauge`

## Purpose
Show the three numeric controls together and how to wire them:
- `Slider` — continuous `0..100` with tick frequency 10; drives a `Gauge`
- `Gauge` — read-only bar, value 0..100
- `SpinCtrl` — independent `0..1000` editor
- A button maps the spin value to the gauge (mod 101)

## Top-level flow
1. Frame 460×360, 1-field `StatusBar` ("Move the slider — the gauge follows.").
2. `Slider::new(&frame, 0, 100, 30)`; `set_tick_freq(10)`.
3. `Gauge::new(&frame, 100)`; `set_value(30)`.
4. `SpinCtrl::new(&frame, 0, 1000, 250)`.
5. **Slider → Gauge wiring** — `slider.on_value_change(&frame, move || { let v = slider_for_cb.get_value(); gauge_for_cb.set_value(v); status.set_status_text(...) })`.
6. **Button** — `spin.get_value() % 101` → `gauge.set_value(mapped)`.
7. **Spin status** — `spin.on_value_change` updates the status bar.
8. Vertical sizer; `app.run(frame)`.

## Key APIs exercised
- [`Slider::new(&frame, min, max, initial)`](file:///f:/code/ru_wx/ru_wx/src/slider.rs)
- `Slider::set_tick_freq(interval: u32)`
- `Slider::on_value_change(&frame, FnOnce())`
- `Slider::get_value() -> i32`
- [`Gauge::new(&frame, range: u32)`](file:///f:/code/ru_wx/ru_wx/src/gauge.rs)
- `Gauge::set_value(i32)`
- [`SpinCtrl::new(&frame, min, max, initial)`](file:///f:/code/ru_wx/ru_wx/src/spin_ctrl.rs)
- `SpinCtrl::get_value() -> i32`
- `SpinCtrl::on_value_change(&frame, FnOnce())`

## Patterns worth noting
- **Modulo mapping** — the button maps `spin ∈ [0, 1000]` to `gauge ∈ [0, 100]` via `v.rem_euclid(101)` so values > 100 don't blow past the gauge's range.
- **Cloning for closures** — both the slider and the gauge are cloned (`slider_for_cb`, `gauge_for_cb`) so the `on_value_change` closure can read one and write the other.
- **One callback per control** — `Slider` and `SpinCtrl` each have their own `on_value_change`; the closure that updates the status bar is distinct from the slider → gauge wiring closure.

## Win32 notes
- `Slider` → `msctls_trackbar32` (`TRACKBAR_CLASS`) with `TBS_AUTOTICKS` for tick marks.
- `Gauge` → `msctls_progress32` with `PBS_SMOOTH` for the solid bar.
- `SpinCtrl` → `msctls_updown32` paired with an `EDIT` buddy; ru_wx owns the buddy in the same HWND tree.
- `Slider` fires `WM_HSCROLL` with `TB_THUMBPOSITION` / `TB_ENDTRACK`; `SpinCtrl` fires `UDN_DELTAPOS` which ru_wx maps to `on_value_change`.

## Cross-references
- [`slider.md`](file:///f:/code/ru_wx/ru_wx/src/slider.md)
- [`gauge.md`](file:///f:/code/ru_wx/ru_wx/src/gauge.md)
- [`spin_ctrl.md`](file:///f:/code/ru_wx/ru_wx/src/spin_ctrl.md)
- [`button.md`](file:///f:/code/ru_wx/ru_wx/src/button.md)
- [`status_bar.md`](file:///f:/code/ru_wx/ru_wx/src/status_bar.md)
