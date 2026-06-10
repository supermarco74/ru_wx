# `tool_bar.rs` — `ToolBar` (ToolbarWindow32 wrapper)

A wrapper around the Win32 **`ToolbarWindow32`** common control. It is
attached to a `Frame` and provides a flat row of icon buttons with optional
text labels.

## Purpose

- A `ToolBar` is created *attached to a frame*. Unlike the `AuiToolBar`,
  this bar is **not** dockable and **not** draggable — it is the simple
  flat-toolbar idiom.
- Buttons are buffered as a `Vec<ToolSpec>` and committed to the native
  control when `realize()` is called. This is the standard Win32 toolbar
  idiom: stage the data, then ask the control to lay it out.

## Public type

```rust
pub struct ToolBar { /* Rc<RefCell<ToolBarInner>> */ }
```

## Public API

| Method | Purpose |
|---|---|
| `new(frame) -> Self` | Create an empty toolbar attached to `frame`. |
| `set_image_list(&self, image_list)` | Attach an `ImageList` whose icons are referenced by `image_index` in `add_tool`. |
| `add_tool(&self, id, label, image_index)` | Append a tool entry. `id` is the command id used by `on_tool_clicked`; `image_index` is `-1` for no icon. |
| `add_separator(&self)` | Append a vertical separator. |
| `realize(&self)` | Flush the buffered `Vec<ToolSpec>` to the native control and auto-size. |
| `on_tool_clicked(&self, frame, F)` | Register a single `FnMut(u16)` callback that fires with the tool's command id. |
| `hwnd() -> HWND` (cfg windows) | Raw Win32 handle. |

## Quick start

```rust,no_run
use ru_wx::prelude::*;

// 1. Create the bar and attach it to a frame.
let bar = ToolBar::new(&frame);
frame.set_tool_bar(&bar);

// 2. Provide an icon set. The ImageList is the source of pixels;
//    each add_tool specifies a 0-based index into it.
let icons = ImageList::new(16, 16)?;
bar.set_image_list(&icons);

// 3. Stage tools, then commit them with realize().
bar.add_tool(1001, "New",   0);   // image_index = 0, command id = 1001
bar.add_tool(1002, "Open",  1);
bar.add_tool(1003, "Save",  2);
bar.add_separator();
bar.add_tool(1004, "Exit",  3);
bar.realize();

// 4. Wire the click handler. The closure receives the command id
//    (the same one you passed to add_tool) so you can switch on it.
let bar_for_click = bar.clone();
bar.on_tool_clicked(&frame, move |id| {
    match id {
        1001 => println!("new"),
        1002 => println!("open"),
        1003 => println!("save"),
        1004 => bar_for_click.close_window(), // example: close the frame
        _    => {}
    }
});
```

After `realize()`, the bar fills the top of the frame's client area and receives `WM_SIZE` re-layouts from the frame. Clicks arrive as `WM_COMMAND` with `wParam` low-word = command id, sharing the same dispatch path as menu items.

## Win32 notes

- Local **`TBBUTTON`** struct (32 bytes on x86_64), defined to keep layout
  stable across `windows-sys` versions.
- Window class **`ToolbarWindow32`**, styles `WS_CHILD | WS_VISIBLE |
  TBSTYLE_FLAT | TBSTYLE_TOOLTIPS` (flat look + tooltip on hover).
- Mandatory init sequence (before any `TB_ADDBUTTONS`):
  1. `TB_BUTTONSTRUCTSIZE` — tell the control the size of our `TBBUTTON`.
  2. `TB_SETBITMAPSIZE` — set the icon dimensions.
  3. `TB_SETIMAGELIST` — attach the image list.
- `realize()` sends `TB_ADDBUTTONS` for the buffered list and then
  `TB_AUTOSIZE` so the bar fills the top of the frame's client area.
- `TBSTYLE_SEP` marks a separator entry in the `TBBUTTON.fsStyle` field;
  `TBSTATE_ENABLED` is set on every regular button.
- Clicks arrive as `WM_COMMAND` with `wParam` low-word = tool id, so they
  are dispatched through the same `register_command_handler` table as
  menu items.

## Single-FnMut-per-bar pattern

`on_tool_clicked` accepts a single `FnMut(u16)` and dispatches per-id
internally. The internal storage is `Rc<RefCell<Box<dyn FnMut(u16)>>>` — a
shared, mutable callback that the WndProc calls with the command id and
the user closure switches on it. This is the same pattern used by the
`Menu::append*` methods and keeps the per-widget storage footprint small.

## Cross-references

- [`frame.md`](../window/frame.md) — `Frame::set_tool_bar(&ToolBar)` attaches the
  bar; the bar receives its `WM_SIZE` re-layout from the frame.
- [`image_list.md`](../dc/image_list.md) — the icon source for `add_tool`.
- [`aui_tool_bar.md`](aui_tool_bar.md) — the dockable cousin; uses the
  same `TBBUTTON` layout but adds a gripper, a custom floating window,
  and a dock-state callback.
