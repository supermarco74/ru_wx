# `showcase_all.rs` — comprehensive 20-control + accelerator showcase

## Purpose
The **flagship demo**. Showcases all 20 controls ported from
`MIGRATION_STATUS.md` (the per-widget `wxToolTip` port and the
v0.4.0 HiDPI / v0.4.1 accelerator APIs are bonus), in a single
top-level window with a 3-page `Tab` notebook.

## Run
```bash
cargo run --example showcase_all
```

## What it shows
| #  | Control                     | Notes                                                    |
|----|------------------------------|----------------------------------------------------------|
| 1  | `Slider`                    | continuous value input (0..100, freq 10)                |
| 2  | `Gauge`                     | determinate progress, driven by a `Timer`               |
| 3  | `SpinCtrl`                  | numeric stepper (0..1000)                                |
| 4  | `Choice`                    | simple drop-down (no edit)                               |
| 5  | `CheckListBox`              | list of items with per-item checkboxes                   |
| 6  | `DatePickerCtrl`            | calendar popup date chooser                              |
| 7  | `ColourPickerCtrl`          | colour chooser button                                    |
| 8  | `RadioBox`                  | group of radio buttons in a labelled box                 |
| 9  | `StatusBar`                 | 3 fields at the bottom (status / DPI / tab page)        |
| 10 | `ToolBar`                   | icon toolbar with separators                             |
| 11 | `Tab` (`Notebook`)          | tab control that uses `ImageList` icons                  |
| 12 | `Timer`                     | 50 ms tick driving the gauge                             |
| 13 | `Font`                      | custom font via `FontDesc::new(face, size).bold()`       |
| 14 | `MessageDialog`             | modal About box                                          |
| 15 | `BitmapBundle`              | multi-resolution bitmap (HiDPI toolbar icons)            |
| 16 | `ArtProvider`               | system-icon provider (`ArtId::New`, `ArtClient::Menu`)   |
| 17 | `PopupMenu`                 | on-demand popup (different from `Menu`)                  |
| 18 | `MenuItem` check / radio    | checkable / radio menu items                             |
| 19 | `TopLevelWindow` base       | a richer window base than `Frame`                        |
| 20 | `ToolTip`                   | per-widget hover tooltips + global enable                |
| 21 | HiDPI                       | `Frame::dpi() + Frame::scale_factor()` in status field 1 |
| 22 | Keyboard accelerators       | `Accelerator` + `Menu::append_with_shortcut`             |

## Embedded assets
5 inline SVG byte strings (Bootstrap-Icon style, 24×24 viewBox):
- `ICON_NEW`, `ICON_OPEN`, `ICON_SAVE`, `ICON_CUT`, `ICON_COPY`

## Constants
| ID                | Value |
|-------------------|-------|
| `ID_TOOL_NEW`     | 1001  |
| `ID_TOOL_OPEN`    | 1002  |
| `ID_TOOL_SAVE`    | 1003  |
| `ID_TOOL_CUT`     | 1004  |
| `ID_TOOL_COPY`    | 1005  |

## Tab structure
| Page     | Contents                                                              |
|----------|-----------------------------------------------------------------------|
| Lists    | `Choice` + `CheckListBox` + `RadioBox`                                |
| Numeric  | `Slider` + `SpinCtrl` + `Gauge` (driven by a `Timer`)                 |
| Pickers  | `DatePickerCtrl` + `ColourPickerCtrl` + custom-font `StaticText` + popup-menu trigger button |

## Top-level flow
1. `App::new()`.
2. `TopLevelWindow::new(title, 820, 620)` — centred on screen via
   `window.centre(CentreDirection::Screen)`.
3. 3-field `StatusBar` (status / DPI+scale / tab page).
4. 5× `BitmapBundle` (16/20/24 px) → 1× `ImageList` (24×24).
5. `ToolBar` with 5 tools + 1 separator.
6. `ArtProvider::new()` (no overrides here, just demonstrates the API).
7. `Tab` with image-list icons; 3 pages built as Panels with sizers.
8. **Page 1 — Lists & selections**: `Choice`, `CheckListBox`, `RadioBox` with callbacks.
9. **Page 2 — Numeric inputs + progress**: `Slider`, `SpinCtrl`, `Gauge`. A 50 ms `Timer`
   advances the gauge `(v + 1) % 101`; the status field updates every 10 ticks.
10. **Page 3 — Pickers & custom font & popup trigger**: `DatePickerCtrl`,
    `ColourPickerCtrl`, custom `Font` for a "fancy" `StaticText`, and a
    "Show popup menu" button that builds a `PopupMenu` (Cut / Copy / Pin
    to top / About…) and pops it up. The About item opens a
    `MessageDialog`.
11. File / View / Help menus. File menu uses `append_with_shortcut` and
    `append_disabled_with_shortcut` for `Ctrl+N` / `Ctrl+O` / `Ctrl+S` /
    `Ctrl+P` / `Ctrl+Q`. View menu has checkable items (status / tool /
    full screen) and a radio group (100% / 125% / 150%) plus a "Flash
    taskbar" item. Help menu has an "About ru_wx…" `MessageDialog`.
12. `ToolTip` attached to a handful of widgets; `ToolTip::enable(false)`
    to globally disable all tooltips this library owns.
13. Frame sizer: vertical `BoxSizer` with the notebook at proportion 1.
14. `app.run(window.into_frame())` — drops the `TopLevelWindow` wrapper,
    passing the owned `Frame` to the event loop.

## Key APIs exercised
- `App::new()` / `app.run(frame)`
- `TopLevelWindow::new(title, w, h)` + `centre(CentreDirection::Screen)`
  + `request_user_attention(UserAttentionFlags::Default)` + `frame()`
  accessor + `into_frame()` consumer
- `Frame::dpi() -> Dpi` + `Dpi::scale_factor() -> f32`
- `StatusBar::new(parent, n_fields)` + `set_status_text(&str, field_idx)`
- `BitmapBundle::from_svg_bytes(svg, &[(16,16), (20,20), (24,24)])` + `best_for_size`
- `ImageList::new(w, h)` + `add_bitmap(hbitmap)` + `width()` / `height()`
- `ToolBar::new(parent)` + `set_image_list` + `add_tool(id, label, idx)` + `add_separator` + `realize` + `on_tool_clicked(parent, |id| ...)`
- `ArtProvider::new()` (registry; no overrides in this demo)
- `ArtClient::Menu` / `ArtId::New` enums
- `Tab::new(parent)` + `set_image_list` + `add_page_with_image(label, panel, img_idx)` + `add_page(label, panel)` + `on_selection_change(parent, |idx| ...)`
- `Panel::new(parent)` + `set_sizer(sizer)`
- `Choice::new(parent)` + `append` + `set_selection` + `get_selection` + `get_string` + `on_selection_change`
- `CheckListBox::new(parent)` + `append` + `check(idx, bool)` + `get_string` + `on_check_toggle(parent, |idx, checked| ...)`
- `RadioBox::new(parent, label, &[&str])` + `set_selection` + `on_select(parent, |idx| ...)`
- `Slider::new(parent, min, max, initial)` + `set_tick_freq` + `on_value_change` + `get_value`
- `SpinCtrl::new(parent, min, max, initial)` + `on_value_change` + `get_value`
- `Gauge::new(parent, range)` + `set_value(i)` + `get_value`
- `Timer::new(parent)` + `on_tick(FnMut)` + `start(Duration)` / `stop`
- `DatePickerCtrl::new(parent)` + `on_date_change(parent, |d: Option<Date>| ...)` (Date = { year, month, day })
- `ColourPickerCtrl::new(parent)` + `on_change(parent, |c: Colour| ...)` (Colour = { r, g, b })
- `Font::new(FontDesc)` + `FontDesc::new(face, size).bold()` + `StaticText::set_font(&font)`
- `PopupMenu::new()` + `append(label, parent, cb)` + `append_check_item` + `append_separator` + `popup(parent)`
- `MessageDialog::new(parent, title, text, MessageDialogStyle::Ok, MessageBoxIcon::Information)` + `show_modal()`
- `Menu::new(label)` + `append(label, parent, cb)` + `append_with_shortcut(label, Accelerator, parent, cb)` + `append_disabled_with_shortcut` + `append_separator` + `append_check_item` + `append_radio_item` + `check_item(id, bool)`
- `Accelerator::parse("Ctrl+N") -> Option<Accelerator>` (string form, see `src/accelerator.rs` for the grammar)
- `MenuBar::new()` + `menubar.append(menu)` + `frame.set_menu_bar(menubar)`
- `ToolTip::new(text).attach(&widget.as_widget_ref())` + `set_text` + `text()` + `ToolTip::enable(bool)`
- `BoxSizer::new(Orientation::Vertical)` + `add_with_proportion` + `frame.set_sizer`

## Win32 / platform notes
- The frame's manifest declares `PerMonitorV2` awareness (see
  `app.manifest`), so `Frame::dpi()` returns the per-monitor value
  and updates as the user drags the window between monitors.
- `TopLevelWindow` adds `request_user_attention` (calls
  `FlashWindowEx`) and a `centre` helper. It also wraps the frame in
  an `Arc` so the menu/toolbar callbacks can clone the wrapper and
  borrow a fresh `&Frame` inside the closure body.
- `Accelerator::parse("Ctrl+N")` is the human-readable string form.
  The library converts it into the Win32 `ACCEL` table
  (`fVirt | key | cmd`) and registers it via
  `HACCEL CreateAcceleratorTableW`.
- The popup's `popup(frame)` method calls `TrackPopupMenuEx` with
  `TPM_RIGHTBUTTON` and routes the resulting `WM_COMMAND` to the
  per-item callbacks via the frame's existing `WndProc` table.
- `ToolTip::enable(false)` sets a global flag in the `TooltipManager`
  that causes the wrapper to return early before sending `TTM_ADDTOOL`.
  The OS tooltip control is still owned by the top-level window;
  it is simply not populated.

## Cross-references
- See `src/top_level_window.rs` for the wrapper
- See `src/slider.rs` / `src/gauge.rs` / `src/spin_ctrl.rs`
- See `src/choice.rs` / `src/check_list_box.rs` / `src/radio_box.rs`
- See `src/date_picker_ctrl.rs` / `src/colour_picker_ctrl.rs`
- See `src/font.rs` / `src/static_text.rs`
- See `src/popup_menu.rs` / `src/menu.rs` / `src/menu_item`
- See `src/message_dialog.rs`
- See `src/timer.rs`
- See `src/tool_bar.rs` / `src/bitmap_bundle.rs` / `src/image_list.rs`
- See `src/art_provider.rs`
- See `src/tab.rs` / `src/panel.rs`
- See `src/tooltip.rs` for the global enable flag
- See `src/accelerator.rs` for the parse grammar
- See `src/dpi.rs` for the DPI struct
- See `src/status_bar.rs`
