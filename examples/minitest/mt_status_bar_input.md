# mt_status_bar_input.rs

Minitest for [`TextCtrl`](file:///f:/code/ru_wx/ru_wx/src/text_ctrl.rs) → [`StatusBar`](file:///f:/code/ru_wx/ru_wx/src/status_bar.rs) — type into a text field, click a button, see the value appear in the status bar.

This is a **regression test** for the bug where the `StatusBar` fields were computed with the parent frame's client rect = 0×0 (because the frame was not yet shown at construction time) and never re-computed on `WM_SIZE`, so each field ended up a few pixels wide and could only display one character. The fix added a resize handler in `StatusBar::new` that re-applies the field widths on every `WM_SIZE`.

**Run:** `cargo run --example mt_status_bar_input`

## Purpose
1. `TextCtrl::get_value` round-tripping into `StatusBar::set_status_text`
2. A long, deliberately wide payload that would visibly truncate if the field-width fix were missing
3. Pre-baked sample buttons so the test can be exercised without having to type into the input field
4. 4 fields side by side so each field has a clear width target

## Top-level flow
1. Frame 1000×600.
2. 4-field `StatusBar`; field 0 set to `(empty)`, fields 1-3 pre-populated with `Field N — fixed label` so each field's width is visually obvious.
3. **Section 1: free-form input**
   - `StaticText` label: "Type some text, then click \"Set status\":"
   - `TextCtrl::new(&frame, "Hello, world!")` pre-filled with a sample value
4. **Section 2: presets (long payloads)**
   - **"Set status ← input box"** — `TextCtrl::get_value` → `StatusBar::set_status_text(..., 0)`
   - **"Preset: long string (80+ chars, no truncation)"** — writes `"The quick brown fox jumps over the lazy dog (1234567890)"` (~57 chars). Without the fix this would be clipped to one character.
   - **"Preset: short (\"OK\")"** — writes `"OK"` to show that the field accepts short writes too
   - **"Clear status field 0"** — writes `""` to verify the empty-write path
5. **Layout** — vertical `BoxSizer` with all 4 buttons stacked, terminated with `add_spacer(22)` to reserve room for the status bar.
6. `app.run(frame)`.

> The 22 px spacer at the end of the sizer is critical. The status bar is **not** part of the sizer — it is positioned by its own `WM_SIZE` handler via `MoveWindow` — so without the spacer the last button would be laid out on top of the bar.

## Key APIs exercised
- [`TextCtrl::new(&frame, &str)`](file:///f:/code/ru_wx/ru_wx/src/text_ctrl.rs)
- `TextCtrl::get_value() -> String`
- [`StatusBar::new(&frame, n_fields)`](file:///f:/code/ru_wx/ru_wx/src/status_bar.rs)
- `StatusBar::set_status_text(&str, field_idx)`
- [`Button::new(&frame, &str)`](file:///f:/code/ru_wx/ru_wx/src/button.rs)
- `Button::on_click(&frame, FnClosure)` with `move` captures
- [`BoxSizer::vertical()`](file:///f:/code/ru_wx/ru_wx/src/sizer.rs)
- `BoxSizer::add(widget_ref)`
- `BoxSizer::add_spacer(px: i32)`
- [`Frame::set_sizer(sizer)`](file:///f:/code/ru_wx/ru_wx/src/frame.rs)

## Patterns worth noting
- **Clone before capture, share after** — each button clones the shared state (`status`, `input`) into the closure. `StatusBar` is `Clone` so the four buttons can all push into field 0 without `Rc<RefCell<…>>`.
- **Pre-baked buttons replace a typing step** — instead of forcing the test operator to type, the buttons provide deterministic payloads (long / short / empty) so a screenshot or visual inspection can quickly confirm whether the field-width fix is in effect.
- **The 22-px trailing spacer is the layout contract with the status bar** — any example that uses a `StatusBar` inside a sizer-driven `Frame` must reserve vertical space for it; the bar will not "absorb" the spare height automatically.
- **The two failures look different** — without the `WM_SIZE` fix, only the **long** preset looks broken; the **short** preset looks fine because the bug clips each field to ~one char, which happens to be exactly the right width for `"OK"`. Always test with the longest payload you intend to display.

## Win32 notes
- `StatusBar` is a native `msctls_statusbar32`. Initial field widths are computed from the parent frame's client rect; if that rect is `0×0` (because `ShowWindow` has not run yet) the field widths collapse.
- The fix attaches a `WM_SIZE` handler in `StatusBar::new` that re-applies the saved field widths on every resize, so the bar recovers as soon as the frame is shown and resized.
- `TextCtrl::get_value` issues `WM_GETTEXT` with a buffer that grows until it fits the contents, then converts the wide string into a Rust `String`.
- `Button::on_click` registers a `WM_COMMAND` / `BN_CLICKED` filter on the parent frame.

## Cross-references
- [`status_bar.md`](file:///f:/code/ru_wx/ru_wx/src/status_bar.md)
- [`text_ctrl.md`](file:///f:/code/ru_wx/ru_wx/src/text_ctrl.md)
- [`button.md`](file:///f:/code/ru_wx/ru_wx/src/button.md)
- [`sizer.md`](file:///f:/code/ru_wx/ru_wx/src/sizer.md)
- [`frame.md`](file:///f:/code/ru_wx/ru_wx/src/frame.md)
- [`mt_status_bar.md`](file:///f:/code/ru_wx/ru_wx/examples/minitest/mt_status_bar.md) — feature-by-feature tour
- [`mt_status_bar_minimal.md`](file:///f:/code/ru_wx/ru_wx/examples/minitest/mt_status_bar_minimal.md) — bar-only smoke test
