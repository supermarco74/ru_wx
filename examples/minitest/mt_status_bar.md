# mt_status_bar.rs

Minitest for [`StatusBar`](file:///f:/code/ru_wx/ru_wx/src/status_bar.rs) — 4 fields, 4 features exercised by buttons.

**Run:** `cargo run --example mt_status_bar`

## Purpose
Demonstrate the four main things you can do with a `StatusBar`:
1. **Set field text** — write a distinct string into each of the 4 fields via 4 dedicated buttons
2. **Get field text** — read back all 4 fields and show the result in a modal message box
3. **Get field count** — query `get_fields_count()` and show it
4. **Show / Hide** — toggle the visibility of the whole bar

## Top-level flow
1. Frame 560×360.
2. 4-field `StatusBar`; pre-fill fields 0-3 with "Ready" / "Field 1" / "Field 2" / "Field 3".
3. **(1) Set field text** — one button per field writes "ALPHA" / "BETA" / "GAMMA" / "DELTA" into it.
4. **(2) Get field text** — single "Read all 4 fields" button formats `get_status_text(i)` for `i in 0..4` and shows the dump in a modal `message_box` with `MessageBoxStyle::Ok | MessageBoxIcon::Information`.
5. **(3) Get field count** — "Show field count" button reads `get_fields_count()` and shows it in a `message_box`.
6. **(4) Show / Hide toggle** — `set_visible(!is_visible())` — `StatusBar` has direct `is_visible` / `set_visible` shortcuts that delegate to the underlying `Widget` trait. Clicking flips the bar's visibility in place.
7. Stack all 7 buttons in a vertical sizer; `app.run(frame)`.

> Sizers cannot be nested in `ru_wx` — only Widgets can sit inside a sizer — so we keep the layout flat.

## Key APIs exercised
- [`StatusBar::new(&frame, n_fields)`](file:///f:/code/ru_wx/ru_wx/src/status_bar.rs)
- `StatusBar::set_status_text(&str, field_idx)`
- `StatusBar::get_status_text(field_idx) -> String`
- `StatusBar::get_fields_count() -> usize`
- `StatusBar::is_visible() -> bool`
- `StatusBar::set_visible(bool)`
- [`message_box(&frame, &text, &caption, MessageBoxStyle, MessageBoxIcon)`](file:///f:/code/ru_wx/ru_wx/src/message_box.rs)
- `MessageBoxStyle::Ok`
- `MessageBoxIcon::Information`

## Patterns worth noting
- **All field writes go through `set_status_text`** — there is no per-field setter; the field index is the second argument.
- **Read-back uses the same `get_status_text(i)` for every field** — the dump formats each field individually so partial writes are obvious in the dialog.
- **`set_visible(!is_visible())` is the canonical toggle** — `StatusBar` exposes `is_visible` / `set_visible` as inherent methods that shadow the `Widget` trait's, so the toggle is a one-liner.
- **Modal `message_box` for read-only dumps** — using a dialog (rather than the status bar itself) avoids the "I just wrote to the bar" feedback loop.

## Win32 notes
- `StatusBar` is a native `msctls_statusbar32` (`STATUSCLASSNAME`) with `SBARS_SIZEGRIP` and an internal array of `n_fields` parts.
- `set_status_text` issues `SB_SETTEXTW` with the part index; `get_status_text` issues `SB_GETTEXTW` + `SB_GETTEXTLENGTHW` and copies the wide string into a Rust `String`.
- `get_fields_count` reads the cached `n_fields` from ru_wx's wrapper (no Win32 round-trip needed).
- `set_visible` issues `ShowWindow(hwnd, SW_HIDE | SW_SHOW)` on the bar's HWND.

## Cross-references
- [`status_bar.md`](file:///f:/code/ru_wx/ru_wx/src/status_bar.md)
- [`message_box.md`](file:///f:/code/ru_wx/ru_wx/src/message_box.md) — `message_box`, `MessageBoxStyle`, `MessageBoxIcon`
- [`button.md`](file:///f:/code/ru_wx/ru_wx/src/button.md)
