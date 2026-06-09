# `esempio2.rs` — Mini Editor combining 3 new controls

## Purpose
Combines the **three most recently added controls** in a single
mini-editor-style application to show how they cooperate:

1. **AuiToolBar** — dockable toolbar with gripper
2. **ToolTip** — per-widget tooltips with a global enable toggle
3. **StaticText** — read-only descriptive labels

## Run
```bash
cargo run --example esempio2
```

## What it shows
- Full editor-style window: title bar, toolbar, multiline text editor,
  option row, status bar
- All three "new" controls (AuiToolBar, ToolTip, StaticText) interacting
  with pre-existing controls (Frame, StatusBar, BoxSizer, TextCtrl,
  Button, CheckBox, ImageList)
- Programmatic dock / float via buttons
- A "Show tooltips" checkbox that globally enables / disables every
  tooltip in the window
- A `Cycle Dock` button that rotates through Top → Bottom → Left → Right

## Embedded assets
6 inline SVG byte strings (Bootstrap-Icon style, 24×24 viewBox):
- `ICON_NEW`, `ICON_OPEN`, `ICON_SAVE`, `ICON_CUT`, `ICON_COPY`, `ICON_PASTE`

## Constants
| ID                | Value |
|-------------------|-------|
| `ID_TOOL_NEW`     | 2001  |
| `ID_TOOL_OPEN`    | 2002  |
| `ID_TOOL_SAVE`    | 2003  |
| `ID_TOOL_CUT`     | 2004  |
| `ID_TOOL_COPY`    | 2005  |
| `ID_TOOL_PASTE`   | 2006  |

## Top-level flow
1. Build a 760×520 frame.
2. Build a 1-field `StatusBar` with an initial hint.
3. Build 6 `BitmapBundle`s (16/20/24 px) → 1 `ImageList` (24×24).
4. Create the `AuiToolBar` with 6 tools + 1 separator.
5. Wire `on_tool_clicked` and `on_dock_state_change` to update the status bar.
6. Manually position 4 `StaticText` labels (hint, options, toolbar, info).
7. Attach a `ToolTip` to every interactive widget.
8. Add a multiline `TextCtrl` "document" with welcome text.
9. Add an options row: `CheckBox` (show tooltips) + 3 buttons (Float / Dock Top / Cycle Dock).
10. Build a vertical sizer for the bottom controls (not applied — manual
    layout is used because the AuiToolBar reserves the top 28 px).
11. `app.run(frame)`.

## Key APIs exercised
- `AuiToolBar::new(&frame)` + `set_image_list` + `add_tool` + `add_separator` + `realize`
- `aui.on_tool_clicked(&frame, |id| ...)` — single dispatcher
- `aui.on_dock_state_change(|side| ...)` — fires on dock edge change
- `aui.dock_to(AuiDockSide::Top)` / `aui.float_at(x, y)` — programmatic dock
- `ToolTip::new(text).attach(&widget.as_widget_ref())` — per-widget tooltip
- `ToolTip::enable(bool)` — global enable / disable
- `StaticText::new(parent, text)` + manual `set_position` / `set_size`
- `CheckBox::new(parent, text)` + `on_toggle(parent, || {...})` + `is_checked()`
- `Button::new(parent, label)` + `on_click(parent, || {...})`
- `TextCtrl::multiline(parent, initial_text)` + `as_widget_ref().borrow_mut().set_position` / `set_size`
- `BoxSizer::vertical()` + `add(widget)` + `add_stretch(1)` (built but not applied)

## Win32 / platform notes
- The AuiToolBar reserves the top ~28 px of the client area; the rest
  of the content is manually positioned.
- Tooltips share a single `tooltips_class32` child of the top-level
  window — `ToolTip::enable(false)` sets a global flag in the
  `TooltipManager` that causes the wrapper to return early before
  sending `TTM_ADDTOOL`.
- The `Cycle Dock` button uses `Rc<RefCell<u8>>` to track the next side
  across callbacks (a `FnMut` capture that needs interior mutability).

## Cross-references
- See `src/aui_tool_bar.rs` for the AuiToolBar source
- See `src/tooltip.rs` for the ToolTip wrapper
- See `src/static_text.rs` for the labels
- See `src/text_ctrl.rs` for the multiline editor
- See `src/checkbox.rs` and `src/button.rs`
