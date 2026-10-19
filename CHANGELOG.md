# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- **macOS AppKit backend completed** — 39 modules, ~20.5k lines behind the
  `objc2` / `objc2-app-kit` crates, calling AppKit directly with no
  Objective-C shim. Covers Frame, Panel, Button, StaticText, TextCtrl,
  CheckBox, RadioButton, RadioBox, ComboBox, ListBox, Choice, Slider, Gauge,
  SpinCtrl, ToggleButton, BitmapButton, StatusBar, MenuBar, PopupMenu,
  ToolBar, AuiManager/AuiToolBar, Notebook, ListCtrl, TreeCtrl, Grid,
  MessageBox, TextDialog, DirDialog, ColourPickerCtrl, FontDialog, Timer,
  Accelerator, Clipboard, Drag & Drop, ContextMenu, Splitter, HyperlinkCtrl,
  ToolTip and DatePicker, plus DC painting via `CGBitmapContext`.
- **Linux GTK4 backend** — `src/platform/gtk.rs` (~4.8k lines, ~227 public
  entry points) on `gtk4` 0.11 with the `v4_10` feature. Covers Frame, Panel,
  Button, StaticText, TextCtrl, CheckBox, RadioButton, ComboBox, ListBox,
  Choice, Slider, Gauge, SpinCtrl, ToggleButton, MenuBar, ToolBar,
  AuiManager/AuiToolBar, AuiNotebook, ListCtrl, TreeCtrl, Grid, MessageBox,
  Timer, Accelerator, Splitter, HeaderCtrl and ToolTip.
- Graceful degradation on Linux: when GTK cannot initialise (headless CI) the
  backend falls back to `stub_backend` instead of panicking, so the crate stays
  buildable and testable without a display.
- `src/platform/inspector_macos.rs` and `src/platform/inspector_gtk.rs`
  (788 lines) bring the debug inspector to macOS and Linux.
- `HeaderCtrl` as a first-class component, ported to all three platforms.
- Documentation: component-by-component coverage matrix in `README.md`
  with the list of gaps that still fall back to a stub.

### Changed
- Source tree reorganised from a flat `src/` into a modular layout:
  `core`, `controls`, `containers`, `window`, `dialogs`, `chrome`, `dc`,
  `data`, `net`, `io`, `dnd`, `adv`, `debug`.
- `Cargo.toml` keywords and categories now describe the three platforms
  (`gui-cross-platform`, `macos`, `linux`, `appkit`, `gtk4`), plus an explicit
  `rust-version = "1.70"`.
- `winui3_xaml` and `winui3_reactor` are declared as empty features in the
  published manifest. Cargo resolves `path` dependencies even when they are
  `optional`, so the four unpublished `windows-rs-study` crates were removed
  from `[target.'cfg(target_os = "windows")'.dependencies]`: keeping them made
  `cargo build` fail on any fresh clone that lacked that sibling checkout.

### Known gaps
- Linux: Dialog, TextDialog, FileDialog, DirDialog, ColourDialog, FontDialog,
  StatusBar, Clipboard, Caret, PopupMenu, ContextMenu, ScrolledWindow,
  BitmapButton, HyperlinkCtrl, DatePicker, DC painting, Icon/Bitmap and
  Drag & Drop still fall back to stubs.
- macOS: ScrolledWindow, FileDialog and ColourDialog are stubs; the
  underlying AppKit helpers (`run_file_panel`, `run_color_panel`) exist but
  are not wired to the public dialog types.

## [0.7.1] — 2026-07-22

### Added
- `WidgetCommon` trait: `enable`, `is_enabled`, `show`, `hide`, `is_visible`,
  `set_position`, `set_size`, `set_rect`, `get_position`, `get_size`, `id`
  on all 22+ widget types via blanket impls in `common.rs`.
- `palette` module: `color_ref`, `rgb`, `preset::{WHITE,BLACK,GRAY,...}`,
  `set_background_color`, `clear_brush_cache`, `request_redraw`.
- `subclass` module: `SetWindowSubclass`-based WM_CTLCOLOR* framework for
  per-widget text-colour hints. `Hyperlink` now renders with the Win11 accent
  blue via `HYPERLINK_BLUE`.
- `App::new()` now eagerly calls `ensure_class_registered()` and
  `InitCommonControlsEx(ICC_BAR_*)`, fixing invisible windows on first run.
- `accelerator.rs`: proper `fVirt` mapping (MOD_CONTROL→0x08, MOD_ALT→0x10,
  MOD_SHIFT→0x04) and correct `DestroyAcceleratorTable` symbol (without `W`).
- `layout.rs`: `BoxSizer::layout_into(parent, RECT)` for multi-region layout
  without nested sizers.
- `StatusBar::set_part_count`, `set_part_text`, `get_part_text` for
  multi-part status bars.
- `gui_smoke` CI job: launches all examples, captures PNG screenshots via
  `PrintWindow`, uploads as GitHub artifact (14 days retention).
- `scripts/hash_check.ps1`: compares PNG SHA-256 hashes against a baseline
  stored in `scripts/png_hashes.txt` for regression detection.
- `tests/winui3_widgets.rs`: 8 integration tests (handlers constructible,
  accelerator key parse, frame builder, box sizer, widget event clone,
  slider struct, install_handlers null hwnd).

### Fixed
- `Frame::set_on_close` now correctly calls `events::register_close`.
- `Button::on_click` now fires via `events::register_command`.
- `CheckBox::on_click` now fires via WM_CTLCOLORBTN + BM_GETCHECK.
- `Slider::on_change`, `ListBox::on_selection_change`,
  `Choice::on_selection_change`, `ComboBox::on_selection_change`,
  `TextCtrl::on_change`, `Tab::on_page_changed` all now wire through
  `events::register_hwnd`.
- Toolbar style: `CCS_TOP=0x01` + `TBSTYLE_FLAT=0x0800`; added
  `TB_BUTTONSTRUCTSIZE` and `TB_AUTOSIZE` after button insertion.
- `SS_ETCHEDHORZ`/`SS_ETCHEDVERT` constants corrected to `SS_BLACKFRAME`/
  `SS_GRAYFRAME` so Splitter/StaticLine are visible at any size.
- `TreeView` now correctly uses `HTREEITEM` from `Win32::UI::Controls`.
- `StatusBar` SendMessageW arg types corrected (wParam is `usize` not `u32`).

### Changed
- `BoxSizer` now supports `add_sizer` via the new `layout_into` method.
- `Cargo.toml`: `winui3` feature comment expanded to list all widget types
  and note the WinRT/XAML placeholder status.
- `mod.rs`: 15 modules declared with clear doc-comment table.

## [0.7.0] — 2026-07-01

### Added
- Initial WinUI3 backend placeholder (4 files: `app.rs`, `grid.rs`,
  `layout.rs`, `menu.rs`).
- `events.rs` stub with WM_COMMAND dispatcher.

## [0.6.4] — 2026-06-27

### Added
- AuiToolbar demo, RibbonBar demo, FileExplorer demo.
- MDI support (`mdi_demo` example).
- MusicPlayer example with SVG toolbar icons.
- macOS AppKit backend scaffolding (40 files).

### Changed
- Manifest embedding via `build.rs` + `embed-resource` (no mt.exe step).
