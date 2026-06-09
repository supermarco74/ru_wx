# `input_controls_demo.rs` — comprehensive input-controls showcase (tabbed)

## Purpose
A **comprehensive tabbed showcase** of every input-style widget in
`ru_wx`. Four `Tab` pages, each backed by its own `Panel` with a
vertical `BoxSizer`. The page contents are wired to a single
frame-level status label and to two "Show Summary" / "Clear All" buttons.

## Run
```bash
cargo run --example input_controls_demo
```

## What it shows
- `TextCtrl` in 3 modes: single-line, password, multi-line
- `CheckBox` with `on_toggle` callbacks
- `RadioButton` group (`is_group_start = true` on the first)
- `ComboBox` (editable dropdown) with `on_selection_change`
- `ListBox` in single-select and multi-select modes
- `ListCtrl` in `Report` view with multiple columns
- `Tab` + `Panel` split: each page owns its own sizer
- Aggregating state of every control and displaying it via `message_box`
- Resetting all controls to their default values

## Tab structure
| Page      | Contents                                                                   |
|-----------|----------------------------------------------------------------------------|
| Text      | TextCtrl (single, password, multiline) + label                             |
| Choices   | 4 CheckBoxes + 3 RadioButtons (plan) + ComboBox (country) + 3 labels      |
| Lists     | ListBox (single-select city) + ListBox (multi-select hobbies) + ListCtrl (4 columns, 3 rows) |
| Actions   | "Show Summary" + "Clear All" buttons                                       |

## Top-level flow
1. Build a 680×720 frame.
2. Create a `Tab` notebook.
3. For each page: create a `Panel`, add widgets, set sizer, add to tab.
4. Build a frame-level `BoxSizer::vertical` (4 px padding) → tab (proportion 1) + status label.
5. Wire callbacks (all registered on the **frame** — child events are
   forwarded up from the page panels via WM_COMMAND):
   - 4× `CheckBox::on_toggle` → update status label
   - 3× `RadioButton::on_select` → update status label
   - `ComboBox::on_selection_change` → update status label
   - `ListBox::on_selection_change` (single) → update status label
   - `ListBox::on_selection_change` (multi) → update status label
   - `TextCtrl::on_change` (name) → live-update status label
6. "Show Summary" button: read every control's value, build a multi-line
   summary string, display it in a `message_box`.
7. "Clear All" button: reset every control (text, checkboxes, radio
   buttons, combo, single-select list, listctrl).
8. `app.run(frame)`.

## Key APIs exercised
- `App::new()` / `Frame::builder()` / `Tab::new(&frame)` / `Panel::new(&frame)`
- `TextCtrl::new(parent, default)` / `TextCtrl::password(parent, default)` / `TextCtrl::multiline(parent, default)`
- `CheckBox::new(parent, label)` + `set_checked(bool)` + `on_toggle(&frame, || ...)` + `is_checked()`
- `RadioButton::new(parent, label, is_group_start)` + `set_selected(bool)` + `on_select(&frame, || ...)` + `is_selected()`
- `ComboBox::new(parent)` + `append(label)` + `set_selection(idx)` + `get_value() -> String` + `on_selection_change`
- `ListBox::new(parent)` / `ListBox::multi_select(parent)` + `append` + `set_selection` + `get_selection` + `get_selections` + `get_string(idx) -> Option<String>` + `on_selection_change`
- `ListCtrl::new(parent, ListCtrlStyle::Report)` + `insert_column(idx, label, width)` + `insert_item(idx, text) -> row` + `set_item_text(row, col, text)` + `get_selected_item() -> Option<usize>` + `get_item_text(row, col) -> String` + `delete_all_items()`
- `StaticText::new(parent, text)` + `set_label(text)` + `as_widget_ref()`
- `Button::new(parent, label)` + `on_click(&frame, || ...)`
- `message_box(&frame, text, title, MessageBoxStyle::Ok, MessageBoxIcon::Information)`
- `frame.set_sizer(sizer)` + `sizer.add_with_proportion(widget, 1)`

## Win32 / platform notes
- Each `Panel` owns its child widgets; events bubble to the frame via
  `WM_COMMAND` with `wParam` carrying the control id.
- The "Clear All" handler has no API to clear a multi-select `ListBox`
  in the current build — selection state is left as-is (noted inline).
- The multi-line `TextCtrl` height is pinned to 35 px via
  `set_size(0, 35)` so it doesn't dominate the page.
- `step!` macro flushes stderr after every `eprintln!` so logs are
  immediately visible in a piped console.

## Cross-references
- See `src/text_ctrl.rs` for the three text-input modes
- See `src/checkbox.rs` / `src/radio_button.rs`
- See `src/combo_box.rs` / `src/list_box.rs` / `src/list_ctrl.rs`
- See `src/tab.rs` / `src/panel.rs` for the page split
- See `src/message_box.rs` for the modal dialog
- See `src/sizer.rs` (BoxSizer)
