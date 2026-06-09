# AI_QUICKREF.md — Copy-Paste Idioms

A **cheat sheet of the 12 most common `ru_wx` patterns**, written so an AI can copy a snippet, paste it, and only have to look up the per-module MD when it needs a non-default knob. Every snippet is a complete, minimal, compilable example.

> Conventions used below:
> - `use ru_wx::prelude::*;` brings in the typical working set (App, Frame, all common controls, sizers, geometry, fonts). See [`prelude.md`](./prelude.md).
> - `Window` trait is Windows-only; on non-Windows targets it does not exist (everything below is `#[cfg(target_os = "windows")]`-friendly at the type level).
> - `Frame` is `Clone` (wraps `Rc<RefCell<…>>`). Clone it to share with a callback.

---

## 1. The minimal "Hello, window" program

```rust
use ru_wx::prelude::*;

fn main() {
    let app = App::new();
    let frame = Frame::builder()
        .with_title("Hello")
        .with_size(400, 300)
        .build();
    app.run(frame);
}
```

That's it: one `App`, one `Frame` built with the builder, `app.run(frame)` enters the message loop and blocks until the user closes the window. See [`app.md`](./app.md), [`frame.md`](./frame.md).

---

## 2. Add a button + a click handler that updates a label

```rust
use ru_wx::prelude::*;

fn main() {
    let app = App::new();
    let frame = Frame::builder().with_title("Click test").with_size(400, 300).build();

    let label = StaticText::new(&frame, "Click the button!");
    let label_for_click = label.clone();           // clone for the closure

    let button = Button::new(&frame, "Press me");
    button.on_click(&frame, move || {
        label_for_click.set_label("Button clicked!");
    });

    // Layout: vertical stack
    let mut sizer = BoxSizer::vertical();
    sizer.add(label.as_widget_ref());
    sizer.add(button.as_widget_ref());
    frame.set_sizer(sizer);

    app.run(frame);
}
```

Key idioms:
- `label.clone()` produces a `StaticText` that shares the same backing storage — `set_label` on either updates the same UI.
- `button.on_click(&frame, move || { ... })` registers the callback in the frame's command-handler map.
- `as_widget_ref()` returns a `WidgetRef` suitable for adding to any sizer.

See [`button.md`](./button.md), [`static_text.md`](./static_text.md), [`sizer.md`](./sizer.md).

---

## 3. Layout: vertical / horizontal `BoxSizer`

```rust
use ru_wx::prelude::*;

let mut s = BoxSizer::vertical();      // or horizontal()
s.add(label.as_widget_ref());           // default: no expand, no border
s.add_spacer(8);
s.add(button.as_widget_ref());
s.add_stretchable(0.5);                 // pushes everything below it down
frame.set_sizer(s);
```

- `add(widget_ref)` for a child.
- `add_spacer(n)` for a fixed-pixel gap.
- `add_stretchable(proportion)` for an elastic gap (a "spring").

`GridSizer(rows, cols)` / `FlexGridSizer(rows, cols)` exist for tabular layouts — see [`grid_sizer.md`](./grid_sizer.md).

---

## 4. Modal dialogs (pick a file / folder / colour / font / date / single text / choice)

All modal dialogs follow the same shape — `Some(value)` on OK, `None` on cancel:

```rust
use ru_wx::prelude::*;

// Pick a file to open
let file: Option<PathBuf> = FileDialog::new(&frame, "Open file", "", "", "All files\0*.*\0\0")
    .show_modal();
if let Some(path) = file { /* ... */ }

// Pick a folder
let dir: Option<PathBuf> = DirDialog::new(&frame, "Pick folder", None, DIR_DIALOG_DEFAULT_STYLE)
    .show_modal();

// Pick a colour
let colour: Option<Colour> = ColorDialog::new(&frame).show_modal();

// Pick a font
let font: Option<Font> = FontDialog::new(&frame).show_modal();

// Pick a date
let date: Option<Date> = DatePickerDialog::new(&frame, "Pick a date", "Date", Date::today())
    .show_modal();

// Prompt for a string
let name: Option<String> = TextEntryDialog::new(&frame, "Your name?", "Login", "name", "text")
    .show_modal();

// Ask OK / Yes-No
let ans = message_box(&frame.hwnd(), "Are you sure?", "Confirm",
                      MessageBoxStyle::YesNo, MessageBoxIcon::Question);
```

Module map: [`file_dialog.md`](./file_dialog.md), [`dir_dialog.md`](./dir_dialog.md), [`color_dialog.md`](./color_dialog.md), [`font_dialog.md`](./font_dialog.md), [`date_picker_dialog.md`](./date_picker_dialog.md), [`text_entry_dialog.md`](./text_entry_dialog.md), [`message_box.md`](./message_box.md), [`message_dialog.md`](./message_dialog.md), [`single_choice_dialog.md`](./single_choice_dialog.md).

---

## 5. Drop-down list (`Choice`) and editable combo (`ComboBox`)

```rust
use ru_wx::prelude::*;

let pick = Choice::new(&frame);
pick.append("Apple");
pick.append("Banana");
pick.append("Cherry");
pick.set_selection(0);
pick.on_selection_change(&frame, || {
    let i = pick.get_selection().unwrap_or(0);
    println!("Picked index {i}: {}", pick.get_string(i).unwrap_or_default());
});
```

`ComboBox::new(&frame, "default text")` is the **editable** variant — the user can type a value not in the list, and you can read it with `get_value()`. See [`choice.md`](./choice.md), [`combo_box.md`](./combo_box.md).

---

## 6. Checkbox + Radio group + RadioBox

```rust
use ru_wx::prelude::*;

// Single checkbox
let cb = CheckBox::new(&frame, "Enable sound");
cb.on_toggle(&frame, || println!("checked: {}", cb.is_checked()));

// Radio group (high-level: frame around N radios)
let group = RadioBox::new(&frame, "Size", &["S", "M", "L", "XL"], 0, Orientation::Horizontal);
group.on_selection_change(&frame, || println!("size index: {}", group.get_selection()));

// Low-level radios — set is_group_start = true on the FIRST button in each group
let r1 = RadioButton::new(&frame, "Option A", true);
let r2 = RadioButton::new(&frame, "Option B", false);   // same group as r1
let r3 = RadioButton::new(&frame, "Option C", false);
```

See [`checkbox.md`](./checkbox.md), [`radio_button.md`](./radio_button.md), [`radio_box.md`](./radio_box.md).

---

## 7. Drawing on a window with the `Dc` API

```rust
use ru_wx::prelude::*;

frame.register_paint_handler(move |hdc| {
    let mut dc = unsafe { PaintDC::from_hdc(hdc) };
    let red_brush = Brush::new_solid(Colour::RED);
    dc.fill_rect(Rect::new(10, 10, 100, 50), &red_brush);
    let pen = Pen::new_solid(Colour::BLUE, 2.0);
    dc.draw_line(Point::new(0, 0), Point::new(200, 200), &pen);
});
```

- `PaintDC` is the only safe DC for `WM_PAINT`; `MemoryDC` for offscreen; `WindowDC` for any time.
- Always create fresh `Brush` / `Pen` per call (they wrap GDI handles with `Drop`).

See [`dc.md`](./dc.md), [`brush.md`](./brush.md), [`pen.md`](./pen.md), [`geometry.md`](./geometry.md).

---

## 8. Keyboard accelerator (Ctrl+S, etc.)

```rust
use ru_wx::prelude::*;

let save_id = 1001;
frame.register_accelerator(
    Accelerator::new(VirtualKey::S, Modifiers { ctrl: true, ..Default::default() }),
    save_id,
);
frame.register_command_handler(save_id, Box::new(|| { println!("Save!"); }));
```

- Virtual keys live in [`accelerator.md`](./accelerator.md).
- Accelerator bindings are **stored on the frame** and take effect when `app.run(frame)` starts.
- If a menubar is attached, accelerator mutations also refresh the visible menu-item shortcut labels.

See [`accelerator.md`](./accelerator.md), [`menu.md`](./menu.md).

---

## 9. A Timer that fires every N ms

```rust
use ru_wx::prelude::*;

let mut t = Timer::new(&frame, 100);   // 100 ms interval
t.start(move || { println!("tick"); });

// Later:
t.stop();
```

The closure runs on the Win32 message-loop thread. Capture state by `move` (clone it first if it's `Clone`). See [`timer.md`](./timer.md).

---

## 10. Drag & drop files onto a window

```rust
use ru_wx::prelude::*;

frame.set_drop_files_callback(|dropped: DroppedFiles| {
    for path in dropped.paths() { println!("dropped: {}", path.display()); }
});
```

For richer formats (text, URLs, virtual files) use OLE DnD:

```rust
frame.set_ole_drop_callback(|data: OleDroppedData, _pos: OleDropPosition| {
    if let Some(text) = data.text() { println!("text: {text}"); }
    if let Some(urls) = data.urls() { for u in urls { println!("url: {u}"); } }
    OleDropEffect::Copy
})?;
```

See [`drop_target.md`](./drop_target.md), [`ole_dnd.md`](./ole_dnd.md).

---

## 11. Status bar at the bottom of a frame

```rust
use ru_wx::prelude::*;

let sb = StatusBar::new(&frame);
sb.set_status_text("Ready");
sb.push_status_text("Loading…");        // pushes a transient text on top
sb.pop_status_text();                    // pops back to the previous text
```

See [`status_bar.md`](./status_bar.md).

---

## 12. High-DPI: read the scale factor and resize a font

```rust
use ru_wx::prelude::*;

let dpi = frame.dpi();                        // e.g. Dpi(192) for a 200%-scaled display
let scale = frame.scale_factor();             // e.g. 2.0
let big_font = Font::new(FontDesc::default()
    .with_point_size(12.0 * scale));
label.set_font(&big_font);
```

`App::set_process_dpi_awareness(DpiAwareness::PerMonitorV2)` should be called **before** creating the first `Frame` for the best behaviour. See [`dpi.md`](./dpi.md), [`font.md`](./font.md).

---

## 13. Bonus: menus, toolbars, tabs, list views, grids, trees

These follow the same "build a struct, add to frame, optionally set sizer" pattern. Quick pointers:

- **Menu bar**: [`menu.md`](./menu.md) — `MenuBar` → `Menu::new("&File")` → `menubar.append(file_menu)` → `frame.set_menu_bar(menubar)`.
- **Toolbar**: [`tool_bar.md`](./tool_bar.md) — `ToolBar::new(&frame)`, then `tb.add_tool(...)`, then `frame.set_tool_bar(tb)`.
- **Tabs**: [`tab.md`](./tab.md) — `let tab = Tab::new(&frame); tab.add_page("Tab 1", panel1); …; frame.set_sizer(sizer_with_tab)`.
- **List view (multi-column)**: [`list_ctrl.md`](./list_ctrl.md) — `ListCtrl::new(&frame, ListCtrlStyle::report)`, then `lc.insert_column(0, "Name", 200)`, `lc.insert_item(0, "Alice")`.
- **Grid (editable cells)**: [`grid.md`](./grid.md) — `Grid::new(&frame, rows, cols)`, then `grid.set_cell_value(row, col, "x")`.
- **Tree**: [`tree_ctrl.md`](./tree_ctrl.md) — `TreeCtrl::new(&frame)`, then `let root = tree.add_root("Root"); tree.append_item(root, "Child")`.
- **Tray icon**: [`icon_tray.md`](./icon_tray.md) — `IconTray::new(&frame, icon, tooltip)`, then `tray.show()`.
- **Animation (GIF/APNG)**: [`animation.md`](./animation.md) + [`animation_ctrl.md`](./animation_ctrl.md) — `let a = Animation::load_file("logo.gif")?; let ac = AnimationCtrl::new(&frame); ac.set_animation(&a); ac.play()`.
- **OpenGL**: [`gl_canvas.md`](./gl_canvas.md) — `let gl = GLCanvas::new(&frame, 400, 300); gl.set_current(); /* draw with gl11::* */`.
- **Media (audio/video)**: [`media_ctrl.md`](./media_ctrl.md) — `let m = MediaCtrl::new(&frame); m.load("song.mp3")?; m.play()`.

---

## Appendix A — The shape of every widget

Almost every concrete widget type follows the same internal layout:

```rust
pub struct Foo {                              // Clone
    inner: Rc<RefCell<FooInner>>,
}

struct FooInner {                             // private
    hwnd: HWND,                               // underlying Win32 control
    id: u16,                                  // control id used in WM_COMMAND dispatch
    rect: Rect,                               // cached position+size
    enabled: bool, visible: bool,
    // ...widget-specific fields
}
```

This pattern gives you:
- **Cloning is cheap** — clones share the same backing storage.
- **Callbacks can mutate** — they hold a cloned `Rc<RefCell<…>>` and re-borrow.
- **`HWND` is private** — user code cannot accidentally destroy the control.

If you need the raw `HWND` (rare, only for custom FFI), call `widget.hwnd()` on Windows (`window.hwnd()` through the `Window` trait).

See [`widget.md`](./widget.md) for the full `Widget` / `Window` / `WidgetRef` story.

---

## Appendix B — Common error patterns and their fixes

| Symptom | Likely cause | Fix |
| --- | --- | --- |
| Compile error on macOS / Linux | Code names `HWND` or `Window` trait | Wrap with `#[cfg(target_os = "windows")]` |
| Click handler never fires | Forgot to call `on_click(&frame, ...)` with a **cloned** frame/closure that captures a cloned `Rc` | Clone any `Rc<RefCell<…>>` you capture |
| Widget sits at default 0,0 / 100×30 | No sizer installed | `frame.set_sizer(sizer)` after adding all widgets |
| Sizer doesn't re-layout on resize | Did not call `frame.set_sizer` (it installs the resize hook) | Use `set_sizer`, not manual `MoveWindow` |
| `frame.show()` exits immediately | Constructed a non-Windows host | The crate is Win32-only; the example must run on Windows |
| Rust complains about `'static` on a callback | The closure captured a `&T` | Clone the captured `T` and `move` it in |
| A radio button click doesn't deselect siblings | Did not set `is_group_start = true` on the first button of the group | Add `WS_GROUP` to the first radio in each group |

---

## Appendix C — Reading the per-module MDs efficiently

When you open a per-module MD (e.g. `button.md`):

1. Skim the **one-liner** under the title to confirm you have the right module.
2. Scan **`## Key Types`** for the main struct names.
3. Jump to **`## Key Methods`** (or `## Public Methods`) and find the operation you need.
4. If you need to know the underlying Win32 message / style flag, look in **`## Win32 Notes`**.
5. **`## See Also`** at the bottom always points to closely related modules.

The `.rs` file is the source of truth — the MDs are curated summaries. If a method signature disagrees, **trust the `.rs`**.
