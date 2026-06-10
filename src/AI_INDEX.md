# AI_INDEX.md — Task-Oriented Entry Point

This file is the **landing page for an AI agent** that needs to use `ru_wx` to build a Win32 GUI application in Rust. It maps *user-facing tasks* (e.g. "show a file picker", "draw a coloured rectangle") to the **single module** (or two) that implements them, with a copy-paste-ready pointer to the per-module MD for full details.

> **Read order for an AI**:
> 1. [`lib.md`](lib.md) — the crate-level orientation (1 page).
> 2. [`AI_QUICKREF.md`](AI_QUICKREF.md) — copy-paste idioms for the 10 most common patterns.
> 3. This file — find the task you need; jump to the linked module MD for full API.
> 4. The per-module MD you jumped to — full type/method listing, Win32 details, tests, see-also.

---

## Core concepts (read these first)

| Concept | Module |
| --- | --- |
| `App` entry point & message loop | [`app.md`](core/app.md) |
| `Frame` — top-level window, owns dispatch tables | [`frame.md`](window/frame.md) |
| `Widget` and `Window` traits, `WidgetRef` | [`widget.md`](core/widget.md) |
| `Rect`, `Colour`, `Point`, `Size` | [`geometry.md`](core/geometry.md) |
| `BoxSizer`, `GridSizer`, `FlexGridSizer` (layout) | [`sizer.md`](containers/sizer.md), [`grid_sizer.md`](containers/grid_sizer.md) |
| `prelude::*` (one-line imports) | [`prelude.md`](prelude.md) |
| DPI helpers, scaling | [`dpi.md`](core/dpi.md) |
| `Accelerator` (keyboard shortcuts) | [`accelerator.md`](core/accelerator.md) |
| Logging (multi-target, multi-level) | [`log/mod.md`](core/log/mod.md) |
| Win32 helpers (control id, wide strings) | [`platform/win32.md`](platform/win32.md) |

---

## Tasks → Modules

### "I want to display …"

| Task | Module |
| --- | --- |
| A modal text prompt (OK / Yes-No) | [`message_box.md`](dialogs/message_box.md) (one-shot) · [`message_dialog.md`](dialogs/message_dialog.md) (configurable) |
| A modal "pick a file" dialog | [`file_dialog.md`](dialogs/file_dialog.md) |
| A modal "pick a folder" dialog | [`dir_dialog.md`](dialogs/dir_dialog.md) |
| A modal "pick a colour" dialog | [`color_dialog.md`](dialogs/color_dialog.md) |
| A modal "pick a font" dialog | [`font_dialog.md`](dialogs/font_dialog.md) |
| A modal "pick a date" dialog | [`date_picker_dialog.md`](dialogs/date_picker_dialog.md) |
| A modal "type some text" dialog | [`text_entry_dialog.md`](dialogs/text_entry_dialog.md) |
| A modal "choose one" / "choose many" dialog | [`single_choice_dialog.md`](dialogs/single_choice_dialog.md) |
| A modal "find / replace" panel (modeless) | [`find_replace_dialog.md`](dialogs/find_replace_dialog.md) |
| A modal "pick a Unicode symbol" dialog | [`symbol_picker_dialog.md`](dialogs/symbol_picker_dialog.md) |
| A modal "busy" overlay during long ops | [`busy_info.md`](core/busy_info.md) |
| A modal "progress bar" dialog | [`progress_dialog.md`](dialogs/progress_dialog.md) |
| A custom modal / modeless dialog | [`dialog.md`](window/dialog.md) |
| A right-click popup menu | [`popup_menu.md`](window/popup_menu.md) |
| A persistent icon in the system tray | [`icon_tray.md`](chrome/icon_tray.md) |
| A non-modal tooltip on a widget | [`tooltip.md`](core/tooltip.md) |
| A short-lived status message at the bottom | [`status_bar.md`](chrome/status_bar.md) |

### "I want to draw …"

| Task | Module |
| --- | --- |
| A bitmap (decoded from PNG / JPEG / etc.) | [`bitmap.md`](dc/bitmap.md) |
| An image in memory (RGBA buffer) | [`image.md`](dc/image.md) |
| A vector icon (SVG) | [`icon.md`](dc/icon.md) |
| A set of bitmaps keyed by ID (toolbar/list) | [`bitmap_bundle.md`](dc/bitmap_bundle.md) · [`image_list.md`](dc/image_list.md) |
| A themed stock icon (folder, file, save …) | [`art_provider.md`](dc/art_provider.md) |
| Solid fills, lines, dashed strokes | [`brush.md`](dc/brush.md) · [`pen.md`](dc/pen.md) |
| Direct GDI drawing on a window | [`dc.md`](dc/dc.md) |
| An animated GIF / APNG | [`animation.md`](adv/animation.md) + [`animation_ctrl.md`](adv/animation_ctrl.md) |
| An OpenGL rendering surface | [`gl_canvas.md`](dc/gl_canvas.md) |
| Audio / video playback (MCI) | [`media_ctrl.md`](adv/media_ctrl.md) |
| An image that just sits there | [`static_bitmap.md`](controls/static_bitmap.md) |
| A static text label | [`static_text.md`](controls/static_text.md) |
| A horizontal / vertical divider | [`static_line.md`](controls/static_line.md) |
| A labelled box border | [`static_box.md`](controls/static_box.md) |

### "I want a control that the user can interact with …"

| Task | Module |
| --- | --- |
| A push-button (text or icon) | [`button.md`](controls/button.md) · [`bitmap_button.md`](controls/bitmap_button.md) · [`toggle_button.md`](controls/toggle_button.md) |
| A two-state check-box | [`checkbox.md`](controls/checkbox.md) |
| A list with check-boxes per item | [`check_list_box.md`](controls/check_list_box.md) |
| A single radio button (low-level) | [`radio_button.md`](controls/radio_button.md) |
| A radio-button group (high-level) | [`radio_box.md`](controls/radio_box.md) |
| A drop-down pick list (no edit) | [`choice.md`](controls/choice.md) |
| A drop-down combo (editable) | [`combo_box.md`](controls/combo_box.md) |
| A scrollable list of strings | [`list_box.md`](controls/list_box.md) |
| A multi-column list view | [`list_ctrl.md`](controls/list_ctrl.md) |
| A tree view (collapsible hierarchy) | [`tree_ctrl.md`](controls/tree_ctrl.md) |
| A table of editable cells | [`grid.md`](containers/grid.md) |
| A single-line or multi-line text box | [`text_ctrl.md`](controls/text_ctrl.md) |
| A numeric spin box (integer) | [`spin_ctrl.md`](controls/spin_ctrl.md) |
| A numeric spin box (float) | [`spin_ctrl_double.md`](controls/spin_ctrl_double.md) |
| Up / down arrows only | [`spin_button.md`](controls/spin_button.md) |
| A horizontal / vertical slider | [`slider.md`](controls/slider.md) |
| A progress bar | [`gauge.md`](controls/gauge.md) |
| A horizontal / vertical scroll bar | [`scroll_bar.md`](containers/scroll_bar.md) |
| A date picker (in-place) | [`date_picker_ctrl.md`](controls/date_picker_ctrl.md) |
| A colour picker (in-place) | [`colour_picker_ctrl.md`](controls/colour_picker_ctrl.md) |

### "I want to organise a window …"

| Task | Module |
| --- | --- |
| A top-level window | [`frame.md`](window/frame.md) |
| An enhanced top-level (iconify / full-screen) | [`top_level_window.md`](window/top_level_window.md) |
| A child panel with its own WndProc | [`panel.md`](window/panel.md) |
| A scrolled panel (auto scrollbars) | [`scrolled_window.md`](containers/scrolled_window.md) |
| A two-pane splitter with draggable sash | [`splitter_window.md`](containers/splitter_window.md) |
| A tabbed notebook | [`tab.md`](containers/tab.md) |
| A toolbar of icons | [`tool_bar.md`](chrome/tool_bar.md) |
| A docking toolbar (AUI) | [`aui_tool_bar.md`](chrome/aui_tool_bar.md) |
| A menubar with cascading menus | [`menu.md`](window/menu.md) |
| A status bar at the bottom | [`status_bar.md`](chrome/status_bar.md) |
| A vertical/horizontal box of widgets | [`sizer.md`](containers/sizer.md) |
| A grid of widgets | [`grid_sizer.md`](containers/grid_sizer.md) |

### "I want to receive input events …"

| Task | Module |
| --- | --- |
| Keyboard shortcuts (global within the frame) | [`accelerator.md`](core/accelerator.md) |
| A periodic timer (poll / timeout) | [`timer.md`](core/timer.md) |
| Files dropped onto the window (Shell) | [`drop_target.md`](dnd/drop_target.md) |
| OLE COM drag-and-drop (text, URLs, files) | [`ole_dnd.md`](dnd/ole_dnd.md) |

### "I want to set the look …"

| Task | Module |
| --- | --- |
| Create / measure / draw with a font | [`font.md`](core/font.md) |
| Detect / scale for high-DPI displays | [`dpi.md`](core/dpi.md) |
| Use a system theme icon | [`art_provider.md`](dc/art_provider.md) |
| Customise window background | [`frame.md`](window/frame.md) (`set_background_colour`) |

### "I want to log / debug …"

| Task | Module |
| --- | --- |
| Multi-level, multi-target logging | [`log/mod.md`](core/log/mod.md) |
| Pretty-print a log record | [`log/formatter.md`](core/log/formatter.md) |
| Log levels + filtering | [`log/levels.md`](core/log/levels.md) |
| Log target (where output goes) | [`log/target.md`](core/log/target.md) |
| Log record (one entry) | [`log/record.md`](core/log/record.md) |
| Public log API guard | [`log/api_guard.md`](core/log/api_guard.md) |
| Internal access guard | [`log/guards.md`](core/log/guards.md) |
| Log manager (singleton) | [`log/manager.md`](core/log/manager.md) |
| Translate a Win32 error code | [`log/win32_error.md`](core/log/win32_error.md) |

### "I want to talk to Win32 directly …"

| Task | Module |
| --- | --- |
| Allocate control ids, encode wide strings | [`platform/win32.md`](platform/win32.md) |
| Cross-platform stub re-exports | [`platform/mod.md`](platform/mod.md) |

---

## Common cross-cutting concerns

- **Layout**: every widget returns `as_widget_ref() -> WidgetRef` so it can be added to a `BoxSizer` / `GridSizer`. The frame's `set_sizer` installs the sizer and re-layouts on resize.
- **Events**: most controls expose `on_click` / `on_change` / `on_toggle` / … methods that take a closure and register a handler on the frame's `command_handlers` / `notify_handlers` map. Callbacks must outlive the frame (clone any captured state).
- **Cloning**: widgets are `Clone` (they wrap `Rc<RefCell<…>>`). Cloning is the standard way to share a widget between a sizer and a callback closure.
- **HWND access**: on Windows only, every control has a private `HWND`. The `Window` trait (`ru_wx::Window` / `ru_wx::prelude::Window`) is `#[cfg(target_os = "windows")]` and exposes `fn hwnd(&self) -> HWND`. Non-Windows targets have no `HWND`; code that touches it must be `cfg`-gated.
- **Cross-platform stubs**: every Win32-backed type has a non-Windows stub. The stub compiles, the API is reachable, but FFI calls return zero / no-op. This keeps `cargo check` green on macOS / Linux even though `cargo run` only works on Windows.
- **Safety**: every `unsafe` block in the crate is annotated with a `// SAFETY:` comment explaining why the call is valid.

---

## Finding your way around a new module

When you open any per-module MD (e.g. `button.md`), expect this layout:

1. **`# filename.rs`** title + one-liner.
2. **`## Purpose`** — what the module is for and which wxWidgets widget it mirrors.
3. **`## Key Types`** — the public structs/enums, fields where it helps.
4. **`## Key Methods` / `## Public Methods`** — method signatures, grouped by theme.
5. **`## Win32 Notes`** — the underlying Win32 class, styles, message codes.
6. **`## Cross-platform`** — behaviour on non-Windows targets.
7. **`## Tests`** — the unit tests (for verifying behaviour without a real Win32 pump).
8. **`## See Also`** — cross-links to related modules.

If a method is missing a description, the **doc comment on the method itself** in the corresponding `.rs` file is the source of truth.
