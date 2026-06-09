# `aui_tool_bar.rs` — `AuiToolBar` (dockable toolbar with floating window)

The most complex non-`Frame` module in the crate. Implements a **dockable
toolbar** (Top / Bottom / Left / Right) with a **gripper** and a custom
**floating window** for the un-docked state.

## Purpose

- A dockable toolbar: the user can drag the gripper to "tear off" the
  toolbar, which becomes a free-floating top-level window. Double-clicking
  the floating window's title bar (or closing it) re-docks it to the
  top of the frame.
- Uses the same `TBBUTTON` layout as [`ToolBar`](tool_bar.md), plus a
  **`STATIC`** child "gripper" with a `≡` glyph.
- Provides a `dock_state_change` callback so the user can re-layout
  their frame's sizer when the toolbar changes side.

## Public types

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuiDockSide { Top, Bottom, Left, Right, Floating }

pub struct AuiToolBar { /* Rc<RefCell<AuiToolBarInner>> */ }
```

`AuiDockSide::Floating` is reported as the current side when the toolbar
is undocked; `dock_side()` returns the value the user most recently set
via `dock_to` (or `Floating` after a successful `float_at`).

## Public API

| Method | Purpose |
|---|---|
| `new(frame) -> Self` | Create an empty AuiToolBar attached to `frame`, initially docked to `Top`. |
| `set_image_list(&self, image_list)` | Attach the icon image list. |
| `add_tool(&self, id, label, image_index)` | Append a tool entry (same as `ToolBar`). |
| `add_separator(&self)` | Append a vertical separator. |
| `realize(&self)` | Flush the buffered tool list to the native control. |
| `dock_to(&self, side: AuiDockSide)` | Programmatically move the toolbar to a side (or float it via `Floating` + `float_at`). |
| `dock_side(&self) -> AuiDockSide` | Read the current side. |
| `is_floating(&self) -> bool` | Shorthand for `dock_side() == Floating`. |
| `float_at(&self, x, y)` | Tear off the toolbar and show the floating window at `(x, y)`. |
| `on_dock_state_change<F: FnMut(AuiDockSide) + 'static>(&self, F)` | Register a side-change callback. |
| `on_tool_clicked<F: FnMut(u16) + 'static>(&self, frame, F)` | Register the click callback. |
| `hwnd() -> HWND` (cfg windows) | The toolbar's own HWND (the one inside the frame or inside the floating window). |
| `as_widget_ref(&self) -> WidgetRef` | For sizer interop. |

## Quick start

A complete, copy-pasteable example that builds a 3-tool AuiToolBar with
an image list, wires up the click callback, docks it to the left, and
reacts to a programmatic dock-side change.

```rust,no_run
use ru_wx::prelude::*;

fn build_toolbar(frame: &Frame) -> AuiToolBar {
    // 1. Create the toolbar (initially docked to Top).
    let bar = AuiToolBar::new(frame);

    // 2. Attach an image list (icons 0..n). The image list owns its HBITMAPs.
    let icons = ImageList::new(16, 16)?;
    // icons.add_icon_from_svg_bytes(include_bytes!("../assets/icons/file-new.svg"))?;
    // icons.add_icon_from_svg_bytes(include_bytes!("../assets/icons/folder-open.svg"))?;
    // icons.add_icon_from_svg_bytes(include_bytes!("../assets/icons/exit.svg"))?;
    bar.set_image_list(icons);

    // 3. Append tools, then realize() to flush to the native control.
    bar.add_tool(1001, "New",  0);
    bar.add_tool(1002, "Open", 1);
    bar.add_separator();
    bar.add_tool(1003, "Exit", 2);
    bar.realize();

    // 4. React to clicks (the id you passed to add_tool).
    bar.on_tool_clicked(frame, |id| {
        match id {
            1001 => println!("new"),
            1002 => println!("open"),
            1003 => println!("exit"),
            _    => {}
        }
    });

    // 5. React to dock-side changes (user dragged gripper, double-clicked
    //    the floating title bar, etc.).
    bar.on_dock_state_change(|side| {
        println!("toolbar side changed to {:?}", side);
    });

    // 6. Programmatic docking.
    bar.dock_to(AuiDockSide::Left);

    // 7. Float it explicitly at a screen position.
    // bar.float_at(200, 200);

    bar
}
```

**Typical workflow**

1. Create the bar with `AuiToolBar::new(frame)`. It is initially docked to
   `AuiDockSide::Top`.
2. Attach an `ImageList` via `set_image_list`. Tool icons are referenced
   by zero-based index into that list.
3. Append tools with `add_tool(id, label, image_index)`, optional
   `add_separator()` between groups, then call `realize()` to commit the
   buffered entries to the native control.
4. Register a click callback with `on_tool_clicked(frame, |id| ...)` and
   an optional dock-side callback with `on_dock_state_change(|side| ...)`.
5. Re-dock programmatically with `dock_to(AuiDockSide::*)`; float with
   `float_at(x, y)`. Use `is_floating()` / `dock_side()` to read state.
6. Hand the toolbar to a sizer via `as_widget_ref()`. The bar will own its
   own dock-side rectangle after `dock_to`, but still benefits from a
   sizer that resizes the rest of the frame around it.

**Notes**

- The bar always has a **`≡`** gripper on the leading edge. Drag it to
  tear off the bar (it becomes a floating top-level window with
  `WS_EX_TOOLWINDOW`). Double-click the floating title bar (or close it)
  to re-dock to `Top`.
- The `on_dock_state_change` callback is **re-entrancy safe** (take/call/put)
  — you can call `dock_to` from inside the callback.
- Tools share their `id` namespace with `frame` command handlers. Pick
  ids in a range that does not collide with menu command ids.
- The floating window is a **transient** top-level: no taskbar entry, no
  Alt-Tab listing. Closing it (X button) does not destroy the toolbar; it
  re-docks to `Top`.

## Win32 notes

- Constants: **`GRIPPER_WIDTH = 16`**, **`TOOLBAR_HEIGHT = 28`**.
- The gripper is a **`STATIC`** child with style
  `SS_CENTER | SS_NOTIFY | WS_BORDER` displaying the glyph **`"≡"`**
  (U+2261, "Identical To"). The font used is **Segoe UI Symbol**, so
  the Unicode glyph renders correctly.
- When docked, the toolbar is a child of the frame, positioned at the
  side returned by `dock_side()`. The gripper sits to the left (or
  above, for vertical sides) of the toolbar.
- When floating, the toolbar and gripper are re-parented to a custom
  **WS_POPUP | WS_CAPTION | WS_THICKFRAME | WS_SYSMENU | WS_VISIBLE**
  window with the **`WS_EX_TOOLWINDOW`** extended style (no entry in
  the taskbar, no Alt-Tab listing).

## The custom floating window

The floating window uses a **dedicated window class**
**`"RuWxAuiFloating"`**, registered via a `OnceLock<WNDCLASSEXW>` to
guarantee the class is registered exactly once per process.

Its WndProc handles:

- **`WM_NCLBUTTONDBLCLK`** — double-click on the title bar → re-dock
  to `Top`.
- **`WM_CLOSE`** — user clicked the X → re-dock to `Top` (we do **not**
  destroy the toolbar; we just put it back in the frame).
- **`WM_SIZE`** — resize the gripper and toolbar to fit the new
  client area.
- **`WM_DESTROY`** — clear `GWLP_USERDATA` so the raw-pointer raw-load
  in the next WndProc invocation doesn't read freed memory.

The `SetWindowLongPtrW(floating, GWLP_USERDATA, ...)` pattern is used
here with a **raw, non-refcounted pointer** to the inner state — the
floating window is transient and lifetime-coupled to the
`AuiToolBarInner`. This is the same raw-pointer pattern as the tray
balloon window, not the refcounted pattern used by the frame's main
WndProc.

## `do_float` and `do_dock` — free functions

Two free functions implement the actual `SetParent` / `MoveWindow`
plumbing:

- `do_float(inner)` — get the cursor position, create the floating
  window, `SetParent` the gripper and the toolbar to the floating
  window, lay them out.
- `do_dock(inner, side)` — `SetWindowLongPtrW(0)` (clear `GWLP_USERDATA`)
  on the *floating window* **before** `DestroyWindow` to avoid the
  WndProc reading a stale pointer during teardown, then `SetParent` the
  gripper and toolbar back to the frame, `MoveWindow` per the requested
  side.

## Callback ordering

`on_dock_state_change` fires the user callback from
`fire_dock_state_change` which **takes** the callback out of the
`RefCell`, calls it, then puts it back — the same re-entrancy-safe
pattern as `Timer` and `Tab`. This lets the user's callback mutate
the same `AuiToolBar` (e.g. switch to a different side) without
deadlocking.

## Cross-references

- [`frame.md`](frame.md) — the toolbar's parent HWND; the
  `register_command_handler` mechanism is reused for tool clicks.
- [`image_list.md`](image_list.md) — icon source for `add_tool`.
- [`tool_bar.md`](tool_bar.md) — simpler non-dockable cousin; shares
  the `TBBUTTON` layout and the `set_image_list` / `add_tool` /
  `add_separator` / `realize` surface.
- [`icon_tray.md`](icon_tray.md) — sibling custom-window user of
  `SetWindowLongPtrW(GWLP_USERDATA)` with a raw, non-refcounted
  pointer pattern.
