# mt_choice_combo.rs

Minitest for the two drop-down selection controls: [`Choice`](file:///f:/code/ru_wx/ru_wx/src/choice.rs) (read-only) and [`ComboBox`](file:///f:/code/ru_wx/ru_wx/src/combo_box.rs) (editable).

**Run:** `cargo run --example mt_choice_combo`

## Purpose
Show the two text-list dropdowns side-by-side and contrast their read APIs:
- `Choice` → `get_selection()` returns `Option<usize>`; `get_string(idx)` returns `Option<String>`
- `ComboBox` → `get_value()` returns `String` (always — the typed text or the picked entry)

A button reads the current `ComboBox` value on demand and shows it in the status bar.

## Top-level flow
1. Frame 460×320, 1-field `StatusBar`.
2. `Choice` populated with 5 fruits; initial selection 0; on change → status bar.
3. `ComboBox` populated with 5 colours; initial selection 2.
4. `Button("Show ComboBox value")` → formats `combo.get_value()` and writes to the status bar.
5. Vertical sizer; `app.run(frame)`.

## Key APIs exercised
- `Choice::new(&frame)`, `Choice::append(&str)`, `Choice::set_selection(usize)`, `Choice::get_selection()`, `Choice::get_string(usize) -> Option<String>`
- `Choice::on_selection_change(&frame, FnOnce())`
- `ComboBox::new(&frame)`, `ComboBox::append(&str)`, `ComboBox::set_selection(usize)`
- `ComboBox::get_value() -> String` — editable text, always present

## Patterns worth noting
- **`get_value()` is the canonical "what did the user submit?" call** — works whether the user picked an entry from the dropdown or typed a new value.
- **`get_selection()` / `get_string()` are useful for the read-only `Choice`** — they expose the underlying model index and the original label.
- The button is needed because `ComboBox` has no native "enter pressed" callback in this build.

## Win32 notes
- `Choice` → Win32 `COMBOBOX` with `CBS_DROPDOWNLIST` (no edit field).
- `ComboBox` → Win32 `COMBOBOX` with `CBS_DROPDOWN` (edit field + dropdown list).
- Both send `CBN_SELCHANGE` and `CBN_EDITCHANGE`; ru_wx routes the former to `on_selection_change`.

## Cross-references
- [`choice.md`](file:///f:/code/ru_wx/ru_wx/src/choice.md)
- [`combo_box.md`](file:///f:/code/ru_wx/ru_wx/src/combo_box.md)
- [`button.md`](file:///f:/code/ru_wx/ru_wx/src/button.md)
- [`status_bar.md`](file:///f:/code/ru_wx/ru_wx/src/status_bar.md)
