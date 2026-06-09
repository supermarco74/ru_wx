# ru_wx -- wxWidgets Porting Status

This document tracks the progress of porting wxWidgets to `ru_wx`, a pure-Rust
GUI library with **no C++ bindings** that targets native widgets on each
platform.

| Field | Value |
|---|---|
| Current version | **0.5.7** |
| Edition | 2021 |
| Active back-end | Win32 (Windows) |
| Planned back-ends | AppKit (macOS), GTK (Linux) |
| Source modules | 64 (`src/*.rs`, including `prelude` and `platform/win32.rs`) |
| Public re-exports | ~95 types, traits, and enums in `prelude::*` |
| Examples shipped | 7 main + 32 minitests |
| Big-picture coverage | **~85 %** of the common wxWidgets surface |
| Tests | unit + doctest (all green) |
| Lints | `cargo clippy -- -D warnings` clean (pre-existing lint nits only) |
| Docs | `cargo doc --no-deps` clean |
| Format | `cargo fmt --all -- --check` clean |
| CI | `.github/workflows/ci.yml` runs the full 8-step verification + smoke test |

The 0.3.7 → 0.5.7 jump (eight minor bumps) reflects the addition of:
the full `wxStatic*` family (`StaticLine`, `StaticBox`, `StaticBitmap`),
the scroll / splitter family (`ScrollBar`, `ScrolledWindow`, `SplitterWindow`),
the `wxDC` drawing stack (`Dc` trait, `PaintDC` / `ClientDC` / `WindowDC` /
`MemoryDC`), the GDI primitives (`Pen`, `Brush`, `Bitmap`, `Image`),
the input controls (`Slider`, `Gauge`, `SpinCtrl`, `Choice`,
`CheckListBox`, `DatePickerCtrl`, `ColourPickerCtrl`, `RadioBox`,
`StatusBar`, `ToolBar`, `Timer`, `Font`, `MessageDialog`, `BitmapBundle`,
`ArtProvider`, `PopupMenu`), the `Tab` / `Grid` / `AuiToolBar` containers,
the `IconTray` system-tray type, the `log` subsystem, the
`top_level_window` abstraction, the `prelude` re-export module, and a
single GitHub Actions CI that runs the entire verification pipeline on
every push.

On top of that, the v0.5.8 effort (in progress) adds the navigation /
property / MDI surface from wxWidgets, mirrored here as **11 new public
types across 6 new modules**:

* `book.rs` &mdash; `Listbook`, `Choicebook`, `Treebook`, `Toolbook`
  (four alternative notebook controls that drive pages from a
  `ListBox`, `Choice`, `TreeCtrl`, and `ToolBar` respectively).
* `frame_extras.rs` &mdash; `MiniFrame`, `SplashScreen`, `TipWindow`
  (three special-purpose top-level windows: small caption frame,
  bitmap splash with auto-close timer, transient non-activating popup
  anchored to a control rect).
* `mdi.rs` &mdash; `MDIParentFrame`, `MDIChildFrame` (classic
  multiple-document-interface parent + child, with `MDICLIENT` host,
  cascade / tile / activate helpers).
* `wizard.rs` &mdash; `Wizard`, `WizardPage`, `WizardResult`
  (multi-page Next / Back / Finish / Cancel navigation dialog).
* `property_sheet_dialog.rs` &mdash; `PropertySheetDialog`,
  `PropertySheetDialogResult` (tabbed settings dialog with OK / Cancel
  / Apply buttons).
* `property_grid.rs` &mdash; `Property`, `PropertyGrid`, `PropertyValue`
  (custom-drawn two-column property sheet with String / Int / Float /
  Bool / Choice editors).

The 32 minitests (one focused example per migrated component) live
under `examples/minitest/` and are listed in §1.11.1.

---

## 1. What has been done

All modules below live under `ru_wx/src/` and are re-exported by
[`lib.rs`](../../ru_wx/src/lib.rs).

### 1.1 Core / app

| ru_wx module | wxWidgets equivalent | Status | Notes |
|---|---|---|---|
| `app` | `wxApp` | Done | Win32 message-loop bootstrap, `App::run(frame)` |
| `widget` | `wxWindow` (base) | Done | `Widget` / `WidgetRef` traits; `Window` (Win32-only) |
| `geometry` | `wxPoint` / `wxSize` / `wxRect` / `wxColour` | Done | `Colour::to_colorref` (Windows), inclusive-min / exclusive-max `Rect::contains` |
| `font` | `wxFont` | Done | `FontDesc` builder, `Font::from_desc` |
| `platform` | (internal) | Done | `#[cfg(target_os = "windows")]` `win32.rs`, stub otherwise; per-platform DPI helper |

### 1.2 Top-level windows

| ru_wx module | wxWidgets equivalent | Status | Notes |
|---|---|---|---|
| `frame` | `wxFrame` | Done | `FrameBuilder`, custom wndproc, **tray message dispatch** |
| `panel` | `wxPanel` | Done | |
| `dialog` | `wxDialog` | Done | (no `ShowModal` yet, see roadmap) |
| `top_level_window` | `wxTopLevelWindow` | Done | `set_title`, `set_icon`, `get_title`, `get_icon`, `centre`, full-screen helpers, `UserAttentionFlags`, `WindowCornerPreference` (Win11 `DWMWA_WINDOW_CORNER_PREFERENCE`: `Default` / `DoNotRound` / `Round` / `RoundSmall` via `DwmSetWindowAttribute` / `DwmGetWindowAttribute`) |
| `tab` | `wxNotebook` | Done | Tab control with `add_page` / `select` / `on_selection_change` (no image-list support yet) |
| `frame_extras` | `wxMiniFrame` | Done | Small caption frame (compact top-level window with title bar but no menu / toolbar); own window class, no `FrameData` registry touch |
| `frame_extras` | `wxSplashScreen` | Done | Frame with a centered bitmap and an auto-close `Timer`; `close()` / `bitmap()` |
| `frame_extras` | `wxTipWindow` | Done | Transient non-activating popup (`WS_EX_TOOLWINDOW` + `WS_EX_NOACTIVATE` + `WS_EX_TOPMOST`), anchored to a control rect |
| `mdi` | `wxMDIParentFrame` | Done | Top-level parent with a single `MDICLIENT` child; `cascade_children`, `tile_children`, `close_all_children`, `activate_child` |
| `mdi` | `wxMDIChildFrame` | Done | Child of an `MDIParentFrame` (parented to its `MDICLIENT`, not the parent frame); standard `Frame` API |
| `wizard` | `wxWizard` | Done | Multi-page Next / Back / Finish / Cancel navigation dialog; `WizardPage` (`has_next_page`, `validate`), `WizardResult` enum, local message loop |

### 1.3 Basic controls

| ru_wx module | wxWidgets equivalent | Status | Notes |
|---|---|---|---|
| `static_text` | `wxStaticText` | Done | + `get_label` |
| `static_line` | `wxStaticLine` | Done | Horizontal / vertical separator line |
| `static_box` | `wxStaticBox` | Done | Labelled box container |
| `static_bitmap` | `wxStaticBitmap` | Done | Image display widget |
| `button` | `wxButton` | Done | + `Button::new_with_svg_bytes` (SVG icon), `get_label` |
| `checkbox` | `wxCheckBox` | Done | + `get_label` |
| `radio_button` | `wxRadioButton` | Done | |
| `text_ctrl` | `wxTextCtrl` | Done | + `set_readonly` / `is_readonly`, `set_max_length` / `max_length`, `clear`, `append_text`, `can_undo`, `undo` |
| `slider` | `wxSlider` | Done | + `get_range` returns `(i32, i32)` |
| `gauge` | `wxGauge` | Done | Determinate / indeterminate progress |
| `spin_ctrl` | `wxSpinCtrl` | Done | + `get_range` returns `(i32, i32)` |
| `choice` | `wxChoice` | Done | Simple drop-down (no edit) |
| `check_list_box` | `wxCheckListBox` | Done | ListBox + per-item checkboxes |
| `date_picker_ctrl` | `wxDatePickerCtrl` | Done | Calendar popup, `on_change` filtered for `DTN_DATETIMECHANGE` |
| `colour_picker_ctrl` | `wxColourPickerCtrl` | Done | Colour chooser button |
| `radio_box` | `wxRadioBox` | Done | Group of radio buttons in a box |
| `status_bar` | `wxStatusBar` | Done | 1..N field status bar at the bottom, `set_status_text` |
| `tool_bar` | `wxToolBar` | Done | Icon toolbar with separators |
| `tooltip` | `wxToolTip` | Done | Per-widget tooltips |
| `timer` | `wxTimer` | Done | Repeating / one-shot with `on_tick`; `start`, `stop`, `start_one_shot` |

### 1.4 Lists / grids / trees

| ru_wx module | wxWidgets equivalent | Status | Notes |
|---|---|---|---|
| `list_box` | `wxListBox` | Done | |
| `combo_box` | `wxComboBox` | Done | |
| `list_ctrl` | `wxListCtrl` | Done | + `on_item_selected` filtered for `LVN_ITEMCHANGED` |
| `tree_ctrl` | `wxTreeCtrl` | Done | + `on_selection_change` filtered for `TVN_SELCHANGED` |
| `grid` | `wxGrid` | Done | Sortable / editable cells, `on_selection_changed` |
| `image_list` | `wxImageList` | Done | + `Tab::set_image_list` + `Tab::add_page_with_image` integration (`mt_bitmap_combo` / `mt_tab` with icons) |

### 1.5 Menus / icons / tray / art

| ru_wx module | wxWidgets equivalent | Status | Notes |
|---|---|---|---|
| `menu` / `MenuBar` | `wxMenu` / `wxMenuBar` | Done | `append_with_svg_icon`, `append_disabled`, **`MenuItemKind::Check` / `MenuItemKind::Radio`**, `popup_at_cursor` |
| `icon` | `wxIcon` | Done | SVG (resvg) -> `HICON` |
| `icon_tray` | `wxTaskBarIcon` | Done | `Shell_NotifyIconW`, balloons, context menu |
| `popup_menu` | `wxPopupMenu` | Done | On-demand popup (different from `Menu`) |
| `bitmap_bundle` | `wxBitmapBundle` | Done | Multi-DPI bundles (best-for-HWND selection) |
| `art_provider` | `wxArtProvider` | Done | `ArtId` enum (`New`, `Open`, `Save`, `Cut`, `Copy`, `Paste`, `Undo`, `Redo`, `Find`, `Replace`, `Svg`, `Custom`), system and custom overrides |
| `aui_tool_bar` | `wxAuiToolBar` | Done | Office-style toolbar, `AuiDockSide` enum |

### 1.6 Common dialogs

| ru_wx module | wxWidgets equivalent | Status | Notes |
|---|---|---|---|
| `file_dialog` | `wxFileDialog` | Done | |
| `message_box` | `wxMessageBox` | Done | |
| `message_dialog` | `wxMessageDialog` | Done | Modal message dialog (About box, etc.) |
| `dialog` | `wxDialog` | Done | (no `ShowModal` yet, see roadmap) |
| `property_sheet_dialog` | `wxPropertySheetDialog` | Done | Tabbed settings dialog (Notebook + OK / Cancel / Apply); `add_page`, `add_buttons`, `show_modal` returns `PropertySheetDialogResult` (`Ok` / `Cancel`) |

### 1.7 Layout

| ru_wx module | wxWidgets equivalent | Status | Notes |
|---|---|---|---|
| `sizer` | `wxBoxSizer` | Done | `Orientation` flag, tested proportional math |
| `grid_sizer` | `wxGridSizer` / `wxFlexGridSizer` | Done | |
| `scroll_bar` | `wxScrollBar` | Done | Horizontal / vertical, range + thumb pos, `on_scroll` filtered for `WM_HSCROLL` / `WM_VSCROLL` |
| `scrolled_window` | `wxScrolledWindow` | Done | `SetScrollInfo` / `GetScrollInfo` (`SCROLLINFO` struct), virtual content area |
| `splitter_window` | `wxSplitterWindow` | Done | Vertical / horizontal sash, `set_sash_position`, `split_horizontally` / `split_vertically` |

### 1.8 Drawing / GDI

| ru_wx module | wxWidgets equivalent | Status | Notes |
|---|---|---|---|
| `pen` | `wxPen` | Done | `Pen::new` / `Pen::solid`; `PenStyle` (`Solid`, `Dot`, `Dash`, `Transparent`); `HPEN` RAII |
| `brush` | `wxBrush` | Done | `Brush::new` / `Brush::solid`; `BrushStyle` (`Solid`, `Transparent`); stock-object aware (`NULL_BRUSH`) |
| `bitmap` | `wxBitmap` | Done | 32-bit `DIB` section via `CreateDIBSection`; `from_hbitmap`; `Drop` calls `DeleteObject` |
| `image` | `wxImage` | Done | `Image::load_from_file` (PNG / JPEG / BMP via the `image` crate); `to_bitmap` |
| `dc` | `wxDC` | Done | `Dc` trait + four concrete flavours: `PaintDC` (`BeginPaint` / `EndPaint`), `ClientDC` / `WindowDC` (transient, `ReleaseDC`), `MemoryDC` (off-screen); `set_pen` / `set_brush` / `set_text_color` / `set_bk_color` / `set_bk_mode`, `draw_line` / `draw_rect` / `fill_rect` / `draw_ellipse` / `draw_text` / `draw_text_in_rect` / `draw_bitmap` / `text_extent` |

### 1.9 Other

| ru_wx module | wxWidgets equivalent | Status | Notes |
|---|---|---|---|
| `log` | `wxLog` | Done | Manager, levels (`Fatal` to `Trace`), record, target (`Buffer`, `Chain`, `Null`), `LogFormatter`, Win32 error formatter, `ApiGuard` / `LogNull` |

### 1.10 Book-style notebooks (alternatives to `Tab`)

| ru_wx module | wxWidgets equivalent | Status | Notes |
|---|---|---|---|
| `book::Listbook` | `wxListbook` | Done | Sibling of `Tab`: a vertical `ListBox` on the left drives which `Panel` is visible on the right. Book is *passive* -- the caller wires the `ListBox` selection event to `Listbook::select`. |
| `book::Choicebook` | `wxChoicebook` | Done | Sibling of `Tab`: a `Choice` drop-down on top drives the visible `Panel` below. |
| `book::Treebook` | `wxTreebook` | Done | Sibling of `Tab`: a `TreeCtrl` on the left drives the visible `Panel` on the right (supports hierarchical page labels). |
| `book::Toolbook` | `wxToolbook` | Done | Sibling of `Tab`: a `ToolBar` on top drives the visible `Panel` below. |

All four types share a common `BookCore` and expose the same surface
(`new`, `add_page`, `select`, `current_selection`, `page_count`,
`page_label`, `set_on_selection_change`). They live in a single file
(`book.rs`) to keep the four implementations in lockstep.

### 1.11 Examples shipped

| Example | Demonstrates |
|---|---|
| `examples/window_with_button.rs` | Frame, label, button (plain + SVG), menu, sizer |
| `examples/input_controls_demo.rs` | All basic input widgets in one window |
| `examples/grid_demo.rs` | `Grid` (sortable, editable, multi-row) |
| `examples/icon_tray_demo.rs` | System-tray icon, balloon, context menu, hide/show |
| `examples/showcase_all.rs` | Every public type in one window |
| `examples/aui_toolbar_demo.rs` | `AuiToolBar` with all four dock sides |
| `examples/esempio2.rs` | Italian-language demo: `Slider`, `Gauge`, `SpinCtrl`, `Choice`, `Tab`, `StatusBar`, `Timer` |

#### 1.11.1 Minitests (one focused example per migrated component)

All minitests live under `examples/minitest/` and are built and run
individually with `cargo run --example <name>`.

| Minitest | Component |
|---|---|
| `mt_button` | `Button` (plain + SVG) |
| `mt_checkbox_radio` | `CheckBox`, `RadioButton` |
| `mt_choice_combo` | `Choice`, `ComboBox` |
| `mt_bitmap_combo` | `BitmapComboBox` (Win32 `WC_COMBOBOXEX`, per-row icons) |
| `mt_context_menu` | `PopupMenu`, `Menu`, context-menu dispatch |
| `mt_list_box` | `ListBox` |
| `mt_menu` | `Menu`, `MenuBar` |
| `mt_slider_gauge` | `Slider`, `Gauge` |
| `mt_status_bar` | `StatusBar` |
| `mt_status_bar_input` | `StatusBar` + input controls integration |
| `mt_status_bar_minimal` | Minimal `StatusBar` smoke test |
| `mt_tab` | `Tab` (notebook) |
| `mt_text_ctrl` | `TextCtrl` (single + multi-line) |
| `mt_tree_ctrl` | `TreeCtrl` |
| `mt_static_line` | `StaticLine` (horizontal / vertical separator) |
| `mt_static_box` | `StaticBox` (labelled container) |
| `mt_static_bitmap` | `StaticBitmap` (image display) |
| `mt_splitter` | `SplitterWindow` (horizontal / vertical sash) |
| `mt_scrolled` | `ScrolledWindow` (virtual content area) |
| `mt_scroll_bar` | `ScrollBar` (range + thumb pos) |
| `mt_dc` | `Dc` trait, `MemoryDC`, `ClientDC`, `WindowDC`, paint handler |
| `mt_listbook` | `Listbook` (ListBox-driven notebook) |
| `mt_choicebook` | `Choicebook` (Choice-driven notebook) |
| `mt_treebook` | `Treebook` (TreeCtrl-driven notebook) |
| `mt_toolbook` | `Toolbook` (ToolBar-driven notebook) |
| `mt_mini_frame` | `MiniFrame` (small caption top-level window) |
| `mt_splash_screen` | `SplashScreen` (bitmap + auto-close timer) |
| `mt_tip_window` | `TipWindow` (transient non-activating popup) |
| `mt_mdi` | `MDIParentFrame` + `MDIChildFrame` (multiple-document interface) |
| `mt_wizard` | `Wizard` (multi-page Next / Back / Finish navigation) |
| `mt_property_sheet_dialog` | `PropertySheetDialog` (tabbed settings dialog with OK / Cancel / Apply) |
| `mt_property_grid` | `PropertyGrid` (custom-drawn property sheet with String / Int / Float / Bool / Choice editors) |
| `mt_window_corners` | `WindowCornerPreference` (Win11 `DWMWA_WINDOW_CORNER_PREFERENCE`: apply all 4 corner shapes + read-back round-trip) |

Run with:

```bash
cargo run --example window_with_button
cargo run --example input_controls_demo
cargo run --example grid_demo
cargo run --example icon_tray_demo
cargo run --example showcase_all
cargo run --example aui_toolbar_demo
cargo run --example esempio2
# Minitests
cargo run --example mt_button
cargo run --example mt_checkbox_radio
cargo run --example mt_choice_combo
cargo run --example mt_bitmap_combo
cargo run --example mt_context_menu
cargo run --example mt_list_box
cargo run --example mt_menu
cargo run --example mt_slider_gauge
cargo run --example mt_status_bar
cargo run --example mt_status_bar_input
cargo run --example mt_status_bar_minimal
cargo run --example mt_tab
cargo run --example mt_text_ctrl
cargo run --example mt_tree_ctrl
cargo run --example mt_static_line
cargo run --example mt_static_box
cargo run --example mt_static_bitmap
cargo run --example mt_splitter
cargo run --example mt_scrolled
cargo run --example mt_scroll_bar
cargo run --example mt_dc
cargo run --example mt_listbook
cargo run --example mt_choicebook
cargo run --example mt_treebook
cargo run --example mt_toolbook
cargo run --example mt_mini_frame
cargo run --example mt_splash_screen
cargo run --example mt_tip_window
cargo run --example mt_mdi
cargo run --example mt_wizard
cargo run --example mt_property_sheet_dialog
cargo run --example mt_property_grid
cargo run --example mt_window_corners
```

The `input_controls_demo` example, the `grid_demo` example, and the
`showcase_all` example are the three that the CI smoke-launches on
`windows-latest` after the manifest has been embedded.

### 1.12 Property controls

| ru_wx module | wxWidgets equivalent | Status | Notes |
|---|---|---|---|
| `property_grid` | `wxPropertyGrid` | Done | Custom-drawn two-column property sheet. `Property` (`name` + `PropertyValue`), `PropertyValue` enum (`String` / `Int` / `Float` / `Bool` / `Choice { options, selected }`), `PropertyGrid::new` / `add_property` / `count` / `value` / `on_change`, full keyboard / mouse interaction (LButtonDown toggles edit, Enter commits, Esc cancels, VK_TAB moves focus), three editor kinds (EDIT for String / Int / Float, BUTTON toggle for Bool, COMBOBOX for Choice). |

---

## 2. What is still to be ported

The `wxwin11_demo` (sibling project, using `wxdragon` = the C++ binding for
wxWidgets) is the de-facto reference for the surface that any real application
needs. Everything it uses and we don't have yet is listed below, organised
from "blocking common apps" to "long tail".

### 2.1 Missing controls (medium priority)

| wxWidgets type | Notes |
|---|---|
| `wxSlider` | **DONE (U1–U8)** -- `get_range` added in U2 |
| `wxGauge` | **DONE (U1–U8)** |
| `wxSpinCtrl` | **DONE (U1–U8)** -- `get_range` added in U2 |
| `wxChoice` | **DONE (U1–U8)** |
| `wxCheckListBox` | **DONE (U1–U8)** |
| `wxDatePickerCtrl` | **DONE (U1–U8)** -- `on_change` filtered in U8 |
| `wxColourPickerCtrl` | **DONE (U1–U8)** |
| `wxRadioBox` | **DONE (U1–U8)** |
| `wxStatusBar` | **DONE (U1–U8)** |
| `wxToolBar` | **DONE (U1–U8)** |
| `wxTimer` | **DONE (U1–U8)** -- one-shot added in U8 |
| `wxFont` | **DONE (U1–U8)** |
| `wxMessageDialog` | **DONE (U1–U8)** |
| `wxBitmapBundle` | **DONE (U1–U8)** |
| `wxArtProvider` | **DONE (U1–U8)** |
| `wxPopupMenu` | **DONE (U1–U8)** |
| `wxMenuItem` check/radio | **DONE (U1–U8)** -- `MenuItemKind::Check` / `Radio` |
| `wxTopLevelWindow` base | **DONE (U1–U8)** -- `top_level_window.rs` |
| `wxNotebook` with image list | **DONE** -- `Tab::set_image_list` + `Tab::add_page_with_image` (attached `HIMAGELIST`, `TCM_SETIMAGELIST` / `TCIF_IMAGE`) |
| `wxSpinButton` | **DONE** -- `spin_button.rs` (bare `msctls_updown32`, `UDM_SETRANGE` / `UDM_SETPOS`, `UDS_WRAP`) |
| `wxSpinCtrlDouble` | **DONE** -- `spin_ctrl_double.rs` (up/down + buddy edit, integer-scaled value) |
| `wxToggleButton` / `wxBitmapButton` | **DONE** -- `toggle_button.rs` / `bitmap_button.rs` (`BM_SETCHECK`, `BM_SETIMAGE` with up to 4 state bitmaps) |
| `wxButton::GetDefaultSize` | **DONE** -- `Button::GetDefaultSize() -> (i32, i32)` returns the platform default (88×26 on Windows) |
| `wxBitmapComboBox` | **DONE (v0.5.8+)** -- `BitmapComboBox` in `combo_box.rs` (Win32 `WC_COMBOBOXEX` / `ComboBoxEx32`, `CBEM_SETIMAGELIST` + `CBEM_INSERTITEM`, `CBES_EX_NOEDITIMAGE`; `mt_bitmap_combo`). A true `wxOwnerDrawnComboBox` (custom `WM_DRAWITEM` per row) is not implemented yet — `BitmapComboBox` covers the "row with a small icon" use case that drives most apps. |

### 2.2 Other common controls (medium priority)

| Category | wxWidgets types |
|---|---|
| Scroll / splitter | **DONE** — `wxScrollBar`, `wxScrolledWindow`, `wxSplitterWindow` in §1.7 (`mt_splitter`, `mt_scrolled`, `mt_scroll_bar`) |
| Hyperlink / search | `wxHyperlinkCtrl`, `wxSearchCtrl` |
| Static | **DONE** — `wxStaticBox`, `wxStaticLine`, `wxStaticBitmap` in §1.3 (`mt_static_box`, `mt_static_line`, `mt_static_bitmap`) |
| Lists | `wxSimpleHtmlListBox`, `wxTreeListCtrl` |
| Modern data | `wxDataViewCtrl`, `wxDataViewTreeCtrl`, `wxDataViewListCtrl` |
| Rich / HTML / web | `wxRichTextCtrl`, `wxHtmlWindow`, `wxHtmlEasyPrinting`, `wxWebView` |
| Media / GL | `wxMediaCtrl`, `wxAnimation` / `wxAnimationCtrl`, `wxGLCanvas` |
| Property | **DONE** — `wxPropertyGrid` (§1.12, `mt_property_grid`) and `wxPropertySheetDialog` (§1.6, `mt_property_sheet_dialog`) |
| Wizard | **DONE** — `wxWizard` (§1.2, `mt_wizard`) |
| Other books | **DONE** — `wxListbook`, `wxChoicebook`, `wxTreebook`, `wxToolbook` in §1.10 (`mt_listbook`, `mt_choicebook`, `mt_treebook`, `mt_toolbook`) |
| Special frame | **DONE** — `wxMiniFrame`, `wxSplashScreen`, `wxTipWindow`, `wxMDIParentFrame` / `wxMDIChildFrame` in §1.2 (`mt_mini_frame`, `mt_splash_screen`, `mt_tip_window`, `mt_mdi`) |

### 2.3 Common dialogs (medium priority)

| wxWidgets type | Notes |
|---|---|
| `wxDirDialog` | Folder picker |
| `wxFontDialog` | Font picker |
| `wxColourDialog` | Standard colour picker |
| `wxPrintDialog` / `wxPageSetupDialog` / `wxPrintPreviewFrame` | Printing |
| `wxFindReplaceDialog` | Find/replace |
| `wxProgressDialog` | Modal progress with abort |
| `wxBusyInfo` | Non-blocking "please wait" |
| `wxSingleChoiceDialog` / `wxMultiChoiceDialog` | Pick from a list |
| `wxTextEntryDialog` / `wxPasswordEntryDialog` / `wxNumberEntryDialog` | One-line input |
| `wxSymbolPickerDialog` | Unicode picker |
| `wxDatePickerDialog` (standalone) | Modal calendar |

### 2.4 Drawing / DC (high value for any non-trivial UI)

| wxWidgets type | Notes |
|---|---|
| `wxDC` | **DONE** — `Dc` trait + `PaintDC` / `ClientDC` / `WindowDC` / `MemoryDC` in §1.8 (`mt_dc`) |
| `wxScreenDC` | Whole-screen DC (lower priority than the four already ported) |
| `wxPrinterDC` / `wxPostScriptDC` | Printing DCs |
| `wxBitmap` | **DONE** — `bitmap` in §1.8 |
| `wxImage` | **DONE** — `image` in §1.8 (PNG / JPEG / BMP; TIFF not yet) |
| `wxPen` / `wxBrush` | **DONE** — `pen` / `brush` in §1.8 |
| `wxGraphicsContext` | Anti-aliased vector drawing (Direct2D back-end) |
| `wxAffineMatrix2D` | 2D transforms |
| `wxRegion` | Region ops |

### 2.5 Events (medium priority)

| wxWidgets type | Notes |
|---|---|
| `wxKeyEvent` | Full keyboard handling (we have basic text input only) |
| `wxMouseEvent` | Hover / mouse-wheel / enter-leave |
| `wxFocusEvent` / `wxKillFocusEvent` | |
| `wxPaintEvent` | Custom paint hook (we have no DC yet) |
| `wxSizeEvent` / `wxMoveEvent` | |
| `wxCloseEvent` (with `Veto`) | Confirm-before-close dialogs |
| `wxDropTarget` / drag-and-drop | File-drop, DnD inside the app |
| `wxClipboard` | Copy / paste text & bitmaps |
| `wxAcceleratorTable` / `wxAcceleratorEntry` | Keyboard shortcuts at the table level |
| `wxValidator` | Data-validation framework (forms) |
| `wxContextHelpEvent` | "?" mode help |
| `wxMenuEvent` | Open / close menu |
| `wxNotifyEvent` | Cross-platform notification base |

### 2.6 UI extras (medium priority)

| wxWidgets type | Notes |
|---|---|
| `wxCaret` | Caret in a custom widget |
| `wxPopupWindow` / `wxPopupTransientWindow` | Tooltip-style popups |
| `wxRibbonBar` | Office ribbon (tabbed toolbars) |

### 2.7 System / persistence (low/medium priority)

| wxWidgets type | Notes |
|---|---|
| `wxConfig` (with `wxFileConfig`, `wxRegConfig`) | INI / registry persistence |
| `wxLocale` | i18n |
| `wxStandardPaths` | Documents / AppData / cache paths |
| `wxDirTraverser` | Recursive directory listing |
| `wxFileSystemWatcher` | Watch a folder for changes |
| `wxFileSystem` / virtual FS | zip / http / memory streams |
| `wxSingleInstanceChecker` | Single-instance enforcement |

### 2.8 Threading / IPC (low priority)

| wxWidgets type | Notes |
|---|---|
| `wxThread` / `wxThreadHelper` | Background threads |
| `wxCriticalSection` / `wxMutex` / `wxCondition` / `wxSemaphore` | Synchronisation |
| `wxExecute` | Launch child processes |
| `wxProcess` | Async child process with events |
| `wxSocket` / `wxSocketServer` | TCP / UDP |
| `wxWebRequest` | HTTP(S) |
| `wxMessageQueue` | Cross-thread message passing |
| `wxEvent` cross-thread posting | |

### 2.9 Printing (low priority)

| wxWidgets type | Notes |
|---|---|
| `wxPrintout` | Overridable printing |
| `wxPrinter` | Print job controller |
| `wxPrintPreview` | Print preview frame |
| `wxPageSetupDialog` | Page setup |
| `wxPostScriptDC` | PostScript output |
| `wxPreviewFrame` / `wxPreviewControlBar` | |

### 2.10 Docking / AUI (low priority)

| wxWidgets type | Notes |
|---|---|
| `wxAuiManager` / `wxAuiPaneInfo` | Docking layout |
| `wxAuiNotebook` | Dockable notebook |
| `wxAuiToolBar` | **DONE (U1–U8)** -- `AuiToolBar` + `AuiDockSide` |
| `wxAuiFloatingFrame` | |
| `wxAuiDockArt` | Visual customisation |

### 2.11 Debug / help (low priority)

| wxWidgets type | Notes |
|---|---|
| `wxHelpController` | CHM / HTML help |
| `wxDebugReport` / crash dump | |
| `wxLogWindow` / `wxLogGui` | GUI log sink (we have a non-GUI `log` already, plus a `BufferTarget` for in-process tests) |
| `wxStackWalker` | Stack traces |
| `wxDebugContext` | |

### 2.12 Theming / look & feel (medium priority)

| wxWidgets type | Notes |
|---|---|
| `wxSystemSettings` | Colours, fonts, metrics from the OS |
| `wxAppearance` | Light / dark mode (already used by `wxwin11_demo`) |
| PerMonitorV2 manifest | HiDPI (already in `app.manifest`, honoured by the OS) |
| `wxArtProvider` customisation | **DONE (U1–U8)** |

---

## 3. Cross-platform status

| | Windows | macOS | Linux |
|---|---|---|---|
| Back-end | Win32 (`windows-sys 0.59`) | planned (AppKit) | planned (GTK) |
| Compiles | yes (real impl) | stubs only | stubs only |
| Functional UI | yes | n/a | n/a |
| Lints on stub | n/a | `#[allow(dead_code)]` per stub | `#[allow(dead_code)]` per stub |

The non-Windows path is currently a no-op (`#[cfg(not(target_os = "windows"))]`
stubs in each module). The first multi-platform milestone is to make at least
`app`, `frame`, `panel`, `button`, and `static_text` work on macOS via AppKit.

---

## 4. Build & verify

```bash
cd ru_wx
cargo build --release
cargo build --release --example window_with_button
cargo build --release --example input_controls_demo
cargo build --release --example grid_demo
cargo build --release --example icon_tray_demo
cargo build --release --example showcase_all
cargo build --release --example aui_toolbar_demo
cargo build --release --example esempio2
```

On Windows 11 the example `.exe` files additionally need the Common
Controls v6 / PerMonitorV2 manifest embedded; this is done by the
`build_with_manifest.ps1` wrapper (see U6). On CI this step is
performed automatically by the `smoke_launch_windows` job.

The build emits **zero** warnings on every supported target. The
`cargo clippy --lib --no-deps -- -D warnings` and
`cargo clippy --examples --no-deps -- -D warnings` commands are also
silent.

---

## 5. Roadmap (rough order, no commitment)

1. `wxBitmapButton` / `wxToggleButton` -- small push-button variants
2. **`wxStaticBox` / `wxStaticLine` / `wxStaticBitmap`** -- **DONE** (§1.3, `mt_static_*`)
3. **`wxSplitterWindow` / `wxScrolledWindow` / `wxScrollBar`** -- **DONE** (§1.7, `mt_splitter` / `mt_scrolled` / `mt_scroll_bar`)
4. **`wxDC` family + custom paint** -- **DONE** (§1.8, `mt_dc`); `wxScreenDC` + printing DCs are still pending
5. **`wxNotebook` with `wxImageList`** -- **DONE** (§1.2, `mt_tab` / `mt_bitmap_combo`); also `wxBitmapComboBox` (§1.4, `mt_bitmap_combo`)
6. **`wxListbook` / `wxChoicebook` / `wxTreebook` / `wxToolbook`** -- **DONE** (§1.10, `mt_listbook` / `mt_choicebook` / `mt_treebook` / `mt_toolbook`)
7. **`wxWizard` (multi-page Next / Back / Finish)** -- **DONE** (§1.2, `mt_wizard`)
8. **`wxPropertyGrid` / `wxPropertySheetDialog`** -- **DONE** (§1.12 / §1.6, `mt_property_grid` / `mt_property_sheet_dialog`)
9. **`wxMiniFrame` / `wxSplashScreen` / `wxTipWindow`** -- **DONE** (§1.2, `mt_mini_frame` / `mt_splash_screen` / `mt_tip_window`)
10. **`wxMDIParentFrame` / `wxMDIChildFrame`** -- **DONE** (§1.2, `mt_mdi`)
11. `wxDataViewCtrl` -- modern list/grid
12. `wxValidator` + data binding -- forms
13. `wxRichTextCtrl`, `wxHtmlWindow`, `wxWebView`
14. Printing stack (`wxPrinterDC` / `wxPostScriptDC` / `wxPrintout` / ...)
15. macOS back-end
16. Linux back-end

---

## 6. Glossary

- **ru_wx** -- this library, pure Rust, no C++.
- **wxWidgets** -- the C++ GUI library being ported.
- **wxdragon** -- the C++ binding for wxWidgets (used by `wxwin11_demo`).
- **wxwin11_demo** -- the sibling demo under `wxwin11_demo/` that uses
  `wxdragon` and shows the full set of features a real app needs.
- **prelude** -- `ru_wx::prelude::*` -- a single `use` that brings the
  entire working set into scope (~60 items).
