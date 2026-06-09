# `aui_toolbar_demo.rs` — dockable floating toolbar demo

## Purpose
Showcases the **`AuiToolBar`** widget — a dockable toolbar that the user
can detach into a floating window by dragging its `≡` gripper, then
re-dock by dragging it back or by clicking the gripper again.

## Run
```bash
cargo run --example aui_toolbar_demo
```

## What it shows
- `BitmapBundle::from_svg_bytes(svg, &icon_sizes)` — HiDPI multi-res bitmap
- `ImageList::new(w, h)` + `images.add_bitmap(hb)` — Win32 image list
- `AuiToolBar::new(parent)` + `set_image_list(...)` + `add_tool(id, label, idx)` + `add_separator()` + `realize()`
- `AuiDockSide::{Top, Bottom, Left, Right, Floating}` — current / target dock edge
- `aui.dock_to(side)` / `aui.float_at(x, y)` — programmatic dock / undock
- `on_tool_clicked(parent, |id| {...})` — single dispatcher for every tool
- `on_dock_state_change(|side| {...})` — fires when the dock edge changes

## Embedded assets
Five inline SVG byte strings (Bootstrap-Icon style, 24×24 viewBox):
- `ICON_NEW`, `ICON_OPEN`, `ICON_SAVE`, `ICON_CUT`, `ICON_COPY`

## Constants
| ID                | Value | Tool  |
|-------------------|-------|-------|
| `ID_TOOL_NEW`     | 1001  | New   |
| `ID_TOOL_OPEN`    | 1002  | Open  |
| `ID_TOOL_SAVE`    | 1003  | Save  |
| `ID_TOOL_CUT`     | 1004  | Cut   |
| `ID_TOOL_COPY`    | 1005  | Copy  |

## Top-level flow
1. Build a 760×420 frame.
2. Build 5 `BitmapBundle`s at 16, 20, 24 px (HiDPI).
3. Create a 24×24 `ImageList` and add `bundle.best_for_size((24, 24))` for each icon.
4. Create the `AuiToolBar` (docks itself to the top automatically).
5. Wire `on_tool_clicked` to write a status string.
6. Manually position a `StaticText` label and a few `Button`s below the
   toolbar's reserved 28 px (the dock reserves the top strip).
7. `app.run(frame)`.

## Key APIs exercised
- `BitmapBundle::from_svg_bytes(svg, &[(16,16), (20,20), (24,24)])` —
  rasterises the SVG at all three sizes.
- `bundle.best_for_size((24, 24)) -> Option<RawBitmap>` — picks the
  closest-resolution match.
- `RawBitmap.hbitmap` — the `HBITMAP` field the `ImageList` consumes.
- `AuiToolBar::new(&frame)` — does not go in the frame's sizer; positions itself.
- `set_image_list(&images)` — must be called before `add_tool`.
- `add_tool(id, label, image_index)` — `id` is a `u16` you choose.
- `add_separator()` — vertical divider line.
- `realize()` — finalises the layout (no tools may be added after this).
- `on_tool_clicked(&frame, |id| ...)` — single callback for every tool.
- `on_dock_state_change(|side| ...)` — `side: AuiDockSide`.
- `aui.dock_to(AuiDockSide::Top)` / `aui.float_at(420, 220)`.

## Win32 / platform notes
- The AuiToolBar manages its own `HWND` and intercepts `WM_NCHITTEST` on
  the gripper to begin a drag operation.
- During drag, a **ghost window** is rendered using `UpdateLayeredWindow`
  with the bitmap alpha-masked.
- On drop, the toolbar re-positions its child `HWND` to the new edge
  and updates its `DockSide`; `on_dock_state_change` then fires.
- The frame's main sizer's content area begins at `y = 28` because the
  toolbar owns that top strip.

## Cross-references
- See `src/aui_tool_bar.rs` for the AuiToolBar source
- See `src/bitmap_bundle.rs` for HiDPI bitmap bundles
- See `src/image_list.rs` for the Win32 image-list wrapper
- See `src/static_text.rs` and `src/button.rs` for the labels
