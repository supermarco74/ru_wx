# prelude.rs

One-line "import everything you actually need" re-exports.

## Purpose

The crate exposes ~45 modules and ~80 public types. Most user code only needs the typical "build a window, drop some controls, run the loop" subset. `prelude` gathers exactly that subset so a single `use ru_wx::prelude::*;` brings the working set into scope.

## What it re-exports

Organised by category:

- **Application & top-level windows** — `App`, `Dialog`, `FileDialog`, `Frame`, `FrameBuilder`, `message_box`, `MessageBox*`, `MessageDialog*`, `Panel`, `CentreDirection`, `FullScreenStyle`, `TopLevelWindow`, `UserAttentionFlags`.
- **Common containers** — `AuiDockSide`, `AuiToolBar`, `Menu`, `MenuBar`, `MenuItem`, `MenuItemKind`, `PopupMenu`, `ScrolledWindow`, `ScrollBar`, `ScrollBarOrientation`, `SashEvent`, `SplitterOrientation`, `SplitterWindow`, `StatusBar`, `Tab`, `ToolBar`.
- **Input controls** — `Button`, `CheckListBox`, `CheckBox`, `Choice`, `ColourPickerCtrl`, `ComboBox`, `Date`, `DateFormat`, `DatePickerCtrl`, `BackgroundMode`, `Dc`, `MemoryDC`, `PaintDC`, `WindowDC`, `Gauge`, `ListBox`, `ListCtrl`, `ListCtrlStyle`, `ListItem`, `RadioBox`, `RadioButton`, `Slider`, `SpinCtrl`, `StaticBitmap`, `StaticBox`, `StaticLine`, `StaticLineOrientation`, `StaticText`, `TextCtrl`, `TreeCtrl`, `TreeItem`.
- **Geometry & layout** — `Colour`, `Rect`, `Cell`, `Grid`, `FlexGridSizer`, `GridSizer`, `BoxSizer`, `Orientation`.
- **Image / icon helpers** — `Bitmap`, `BitmapBundle`, `RawBitmap`, `Brush`, `BrushStyle`, `BalloonIcon`, `IconTray`, `Image`, `ImageError`, `Rgba`, `ImageList`, `Pen`, `PenStyle`.
- **Misc helpers** — `Accelerator`, `Modifiers`, `VirtualKey`, `ArtClient`, `ArtId`, `ArtProvider`, DPI helpers, `DroppedFiles`, OLE-DnD types, `Font`, `FontDesc`, `Timer`, `ToolTip`.
- **Always** — `Window` (Windows-only), `Widget`, `WidgetRef`.

## What it deliberately does NOT re-export

Lower-level items that are still reachable at `ru_wx::module_name`:

- The `log` submodule (use `ru_wx::log::*`).
- The `platform` submodule.
- `ArtProvider` (yes, it is in the prelude — listed above for completeness; the deliberate omissions are `RawBitmap` plumbing, `get_*_dpi` free functions, `set_process_dpi_awareness`, etc.).

## Quick start

```rust,no_run
// In the vast majority of user code, this single import is all you need:
use ru_wx::prelude::*;

// It brings in: App, Frame, FrameBuilder, Panel, Button, TextCtrl, StaticText,
// CheckBox, RadioButton, Choice, ComboBox, ListBox, ListCtrl, Slider, Gauge,
// SpinCtrl, StaticBox, StaticLine, StaticBitmap, Sizers (BoxSizer, FlexGridSizer,
// GridSizer), Tab, ScrolledWindow, SplitterWindow, ToolBar, AuiToolBar, Menu,
// MenuBar, MenuItem, PopupMenu, StatusBar, FileDialog, MessageDialog, etc.
//
// You then write a typical "window with a button" example like this:
fn main() {
    let app = App::new();

    let frame = Frame::builder()
        .with_title("Hello")
        .with_size(400, 300)
        .build();

    let panel = Panel::new(&frame);
    let sizer = BoxSizer::new(Orientation::Vertical);
    sizer.add_stretchable(1);
    let btn = Button::new(&panel, "Click me");
    sizer.add(&btn, 0, 0, 0);
    panel.set_sizer(sizer);

    let frame_for_click = frame.clone();
    btn.on_click(move |_| {
        message_box(&frame_for_click, "Hi!", "Greeting", IconType::Info);
    });

    frame.show();
    app.run(frame);
}
```

The import also pulls in the graphics helpers (Colour, Pen, Brush, Font, Bitmap, Image, ImageList) and the timer / tooltip / accelerator conveniences. If you need a lower-level item that isn't here, import it from `ru_wx::module_name` directly.

## Usage

```rust
use ru_wx::prelude::*;
```

## See also

- [`lib.rs`](./lib.md) — full crate-level re-exports.
