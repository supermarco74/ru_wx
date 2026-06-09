# mt_status_bar_minimal.rs

Minitest for [`StatusBar`](file:///f:/code/ru_wx/ru_wx/src/status_bar.rs) — minimal visibility / sizing smoke test.

Creates a frame with a single 4-field status bar and a long preset text in field 0. Used to verify that the status bar renders at the bottom of the frame and is **not** occluded by sibling controls.

**Run:** `cargo run --example mt_status_bar_minimal`

## Purpose
1. Verify a `StatusBar` with 4 fields draws at the bottom of a plain frame
2. Verify field 0 is wide enough to hold a long preset string (the regression covered in `mt_status_bar_input`)
3. Provide a near-zero-noise control case: no buttons, no sizer, no children — if the bar is broken, this is the file to bisect against

## Top-level flow
1. Frame 800×200.
2. 4-field `StatusBar`; field 0 set to `">>> This is field 0 with a long string to verify full text fits <<<"`, fields 1-3 set to `"Field 1"` / `"Field 2"` / `"Field 3"`.
3. No sizer, no other widgets — `app.run(frame)` directly.

> The frame is intentionally **sizer-less**. The bar's own `WM_SIZE` handler positions it at the bottom of the client area regardless of the sizer contents, so this is the cleanest possible repro for the field-width fix.

## Key APIs exercised
- [`StatusBar::new(&frame, n_fields)`](file:///f:/code/ru_wx/ru_wx/src/status_bar.rs)
- `StatusBar::set_status_text(&str, field_idx)`

## Patterns worth noting
- **Use this as the bisection target** — if `mt_status_bar_input` shows clipped fields and you suspect the bar itself, run this. It contains nothing but the bar, so a clip here is a bar bug, not a sizer or sibling-control bug.
- **No `BoxSizer` is required** — `StatusBar` positions itself via its internal `WM_SIZE` handler. You can add it to a bare frame and it will lay out correctly.

## Win32 notes
- `StatusBar` is a native `msctls_statusbar32` with `SBARS_SIZEGRIP`. Without a sizer on the parent, the bar's `WM_SIZE` handler positions it at `(0, client_height - cy)` spanning the full client width.
- `set_status_text` issues `SB_SETTEXTW` with `SBT_NOBORDERS` not set (default bordered parts).
- See `mt_status_bar_input.md` for the full discussion of the `WM_SIZE`-based field-width fix.

## Cross-references
- [`status_bar.md`](file:///f:/code/ru_wx/ru_wx/src/status_bar.md)
- [`mt_status_bar.md`](file:///f:/code/ru_wx/ru_wx/examples/minitest/mt_status_bar.md) — feature tour
- [`mt_status_bar_input.md`](file:///f:/code/ru_wx/ru_wx/examples/minitest/mt_status_bar_input.md) — regression test for the field-width fix
