//! Nome modello scrittore: Composer
//! Sito di riferimento: https://ru_wx.easytaskflow.app/
//!
//! ru_wx - A cross-platform native GUI library for Rust
//!
//! Project site: <https://ru_wx.easytaskflow.app/>
//! Source repository: <https://github.com/supermarco74/ru_wx>
//!
//! This library provides a wxWidgets-like API using native platform controls:
//! - Windows: Win32 API (HWND-based controls)
//! - macOS: AppKit stub backend (placeholder; native bindings planned)
//! - Linux: GTK stub backend (placeholder; native bindings planned)
//!
//! # Example
//! ```no_run
//! use ru_wx::*;
//!
//! let app = App::new();
//! let frame = Frame::builder()
//!     .with_title("Hello")
//!     .with_size(400, 300)
//!     .build();
//!
//! let button = Button::new(&frame, "Click me!");
//! button.on_click(&frame, || {
//!     println!("Clicked!");
//! });
//!
//! app.run(frame);
//! ```
//!
//! # Internal lint policy
//!
//! The crate intentionally allows the
//! `clippy::missing_docs_in_private_items` lint at the crate
//! root. Every public item in the public API is documented
//! (see the module-level rustdoc on each `src/<folder>/*.rs` file
//! and the archived `vecchie_elaborazioni/MIGRATION_STATUS.md`
//! index), but the `pub(crate)` and
//! private items (e.g. the fields of every `*Inner` struct,
//! the Win32 message constants, the helper closures used by
//! `WM_NOTIFY` dispatch) are not, by design. Documenting them
//! would create documentation that is not reachable from the
//! public rustdoc output and would be a maintenance burden for
//! no user-facing benefit. The lint is therefore explicitly
//! silenced at the crate root; individual modules that contain
//! user-facing surface (the `log` submodules, `tooltip::imp`,
//! etc.) carry their own `//!` or `///` rustdoc regardless of
//! the lint suppression.
//!
//! The crate also allows `dead_code` at the root: many
//! public types, fields, and Win32 constants are part of
//! the *API surface* (they are reachable from the rustdoc
//! public-API table of contents) even when no internal
//! call site exercises them yet. This is especially true
//! for the many `WM_*`, `TVGN_*`, `CBEIF_*`, `BM_GET*`,
//! `UDS_*`, `MDICLIENT_*`, `LVS_EX_*`, and similar Win32
//! constants defined for completeness and parity with
//! wxWidgets. Removing the unused public surface to
//! silence the lint would shrink the API and require
//! re-adding items later. The lint is therefore allowed
//! globally; specific in-function issues (unused
//! variables, unnecessary `unsafe` blocks, unused
//! imports) are still fixed per-site.

//! # Module layout
//!
//! The crate surface is organised into ten top-level submodules,
//! grouped by domain:
//!
//! * [`core`] — primitives shared by everything else (`App`,
//!   `Widget`, `Rect`, `Font`, `Dpi`, the `log` system, …).
//! * [`window`] — top-level windows: `Frame`, `Dialog`, `Panel`,
//!   menus, MDI.
//! * [`controls`] — interactive widgets (button, text, list, tree,
//!   grid, slider, …).
//! * [`containers`] — layout helpers (sizers, splitter, scrolled,
//!   tab, book).
//! * [`chrome`] — frame decorations (toolbar, status bar, tray).
//! * [`dialogs`] — modal / modeless dialogs.
//! * [`dc`] — device contexts and drawing primitives.
//! * [`adv`] — advanced / specialised widgets (animation, media,
//!   property grid, wizard).
//! * [`dnd`] — drag-and-drop.
//! * [`platform`] — per-OS backends (Win32 today).
//!
//! The full public API is also re-exported flat at the crate root,
//! so `ru_wx::Frame` works alongside `ru_wx::window::frame::Frame`.

#![allow(clippy::missing_docs_in_private_items)]
#![allow(dead_code)]

pub use crate::core::log;
pub use crate::core::log::{LogBuffer, LogChain, NullTarget};

pub mod adv;
pub mod chrome;
pub mod containers;
pub mod controls;
pub mod core;
pub mod dc;
pub mod dialogs;
pub mod dnd;
pub mod io;
pub mod net;
pub mod platform;
pub mod printing;
pub mod window;

pub use crate::adv::adv_ctrl_events::{AnimationCtrlEvent, MediaCtrlEvent};
pub use crate::adv::html_link_event::HtmlLinkEvent;
pub use crate::adv::rich_text_event::{RichTextEvent, RichTextEventKind};
pub use crate::adv::web_view_event::{WebViewEvent, WebViewEventKind};
pub use crate::adv::animation::{Animation, AnimationFrame};
pub use crate::adv::animation_ctrl::AnimationCtrl;
pub use crate::adv::media_ctrl::{MediaCtrl, MediaState};
pub use crate::adv::help_controller::HelpController;
pub use crate::adv::help_provider::HelpProvider;
pub use crate::adv::simple_help_provider::SimpleHelpProvider;
pub use crate::adv::html_easy_printing::HtmlEasyPrinting;
pub use crate::adv::html_window::HtmlWindow;
pub use crate::adv::log_gui::LogGuiTarget;
pub use crate::adv::log_window::LogWindow;
pub use crate::adv::property_grid::{Property, PropertyGrid, PropertyValue};
pub use crate::adv::property_grid_extras::{
    ColourProperty, FontProperty, PropertyCategory, PropertyColumnSplitter, PropertyGridExtras,
    PropertyHelpStrip,
};
pub use crate::adv::rich_text_ctrl::RichTextCtrl;
pub use crate::adv::rich_text_buffer::RichTextBuffer;
pub use crate::adv::html_tag_handler::HtmlTagHandler;
pub use crate::adv::web_view_handler::WebViewHandler;
pub use crate::adv::rich_text_attr::RichTextAttr;
pub use crate::adv::rich_text_style::RichTextStyle;
pub use crate::adv::property_grid_iterator::PropertyGridIterator;
pub use crate::adv::property_grid_manager::PropertyGridManager;
pub use crate::adv::rich_text_style_sheet::RichTextStyleSheet;
pub use crate::adv::web_view::WebView;
pub use crate::adv::wizard::{Wizard, WizardPage, WizardResult};
pub use crate::chrome::aui_manager::{
    AuiDockArt, AuiFloatingFrame, AuiManager, AuiNotebook, AuiPaneInfo,
};
pub use crate::chrome::aui_tool_bar::{AuiDockSide, AuiToolBar};
pub use crate::chrome::aui_toolbar_event::{AuiToolBarEvent, AuiToolBarEventKind};
pub use crate::chrome::info_bar::{InfoBar, InfoBarMessageType};
pub use crate::chrome::info_bar_event::{InfoBarEvent, InfoBarEventKind};
pub use crate::chrome::ribbon_bar_event::{RibbonBarEvent, RibbonBarEventKind};
pub use crate::chrome::ribbon_bar::RibbonBar;
pub use crate::chrome::ribbon_page::RibbonPage;
pub use crate::chrome::ribbon_panel::RibbonPanel;
pub use crate::chrome::ribbon_button_bar::RibbonButtonBar;
pub use crate::chrome::ribbon_gallery::RibbonGallery;
pub use crate::chrome::ribbon_art_provider::RibbonArtProvider;
pub use crate::chrome::header_ctrl::{HeaderColumn, HeaderCtrl};
pub use crate::chrome::icon_tray::{BalloonIcon, IconTray};
pub use crate::chrome::notification_message::NotificationMessage;
pub use crate::chrome::taskbar_icon_event::{TaskBarIconEvent, TaskBarIconEventKind};
pub use crate::chrome::status_bar::StatusBar;
pub use crate::chrome::tool_bar::ToolBar;
pub use crate::containers::book::{Choicebook, Listbook, Toolbook, Treebook};
pub use crate::containers::grid::{
    BadgeKind, BarStyle, Cell, ColumnAlign, Grid, GridAppearance, GridCellStyle, GridDateFormat,
    NumberFormat, SortOrder, PriorityKind,
};
pub use crate::containers::grid_cell_editor::GridCellEditor;
pub use crate::containers::grid_cell_text_editor::GridCellTextEditor;
pub use crate::containers::grid_cell_number_editor::GridCellNumberEditor;
pub use crate::containers::grid_cell_float_editor::GridCellFloatEditor;
pub use crate::containers::grid_cell_bool_editor::GridCellBoolEditor;
pub use crate::containers::grid_cell_choice_editor::GridCellChoiceEditor;
pub use crate::containers::grid_cell_date_editor::GridCellDateEditor;
pub use crate::containers::grid_cell_renderer::GridCellRenderer;
pub use crate::containers::grid_cell_string_renderer::GridCellStringRenderer;
pub use crate::containers::grid_cell_number_renderer::GridCellNumberRenderer;
pub use crate::containers::grid_cell_bool_renderer::GridCellBoolRenderer;
pub use crate::containers::grid_table::{FunctionGridTable, GridTable};
pub use crate::containers::grid_coords::GridCoords;
pub use crate::containers::grid_range::GridRange;
pub use crate::containers::grid_block::GridBlock;
pub use crate::containers::grid_cell_attr::GridCellAttr;
pub use crate::containers::grid_string_table::GridStringTable;
pub use crate::containers::static_sizer::StaticSizer;
pub use crate::containers::sizer_spacer::SizerSpacer;
pub use crate::containers::grid_icons::GridIcons;
pub use crate::containers::grid_bag_sizer::{GridBagPosition, GridBagSizer};
pub use crate::containers::grid_sizer::{FlexGridSizer, GridSizer};
pub use crate::containers::wrap_sizer::WrapSizer;
pub use crate::containers::scroll_bar::{ScrollBar, ScrollBarOrientation, ScrollEvent as ScrollBarEvent};
pub use crate::containers::scrolled_window::ScrolledWindow;
pub use crate::containers::scrolled_window::ScrollEvent as ScrolledWindowScrollEvent;
pub use crate::containers::scrollable_panel::ScrollablePanel;
pub use crate::containers::sizer::{BoxSizer, Orientation};
pub use crate::containers::sizer_flags::SizerFlags;
pub use crate::containers::sizer_item::SizerItem;
pub use crate::containers::static_box_sizer::StaticBoxSizer;
pub use crate::containers::splitter_window::{SashEvent, SplitterOrientation, SplitterWindow};
pub use crate::containers::data_view::{
    DataViewColumn, DataViewCtrl, DataViewListCtrl, DataViewModel, DataViewRenderer,
    DataViewTreeCtrl, InMemoryDataViewModel, TextRenderer,
};
pub use crate::containers::data_view_bitmap_renderer::DataViewBitmapRenderer;
pub use crate::containers::data_view_choice_renderer::DataViewChoiceRenderer;
pub use crate::containers::data_view_toggle_renderer::DataViewToggleRenderer;
pub use crate::containers::tab::Tab;
pub use crate::controls::bitmap_button::BitmapButton;
pub use crate::controls::bitmap_toggle_button::BitmapToggleButton;
pub use crate::controls::animated_button::AnimatedButton;
pub use crate::controls::button::{BitmapAlign, Button};
pub use crate::controls::button_variants::{AnyButton, ButtonKind, ButtonVariants};
pub use crate::controls::check_list_box::CheckListBox;
pub use crate::controls::activity_indicator::ActivityIndicator;
pub use crate::controls::add_remove_ctrl::AddRemoveCtrl;
pub use crate::controls::calendar_ctrl::CalendarCtrl;
pub use crate::controls::calendar_date_attr::CalendarDateAttr;
pub use crate::controls::checkbox::CheckBox;
pub use crate::controls::collapsible_pane::CollapsiblePane;
pub use crate::controls::collapsible_header_ctrl::CollapsibleHeaderCtrl;
pub use crate::controls::combo_ctrl::ComboCtrl;
pub use crate::controls::generic_dir_ctrl::GenericDirCtrl;
pub use crate::controls::menu_button::MenuButton;
pub use crate::controls::popup_ctrl::PopupCtrl;
pub use crate::controls::v_list_box::VListBox;
pub use crate::controls::control_events::{
    CollapsiblePaneEvent, GaugeEvent, SliderEvent, SpinEvent,
};
pub use crate::controls::rearrange_list::RearrangeList;
pub use crate::controls::command_link_button::CommandLinkButton;
pub use crate::controls::context_help_button::ContextHelpButton;
pub use crate::controls::choice::Choice;
pub use crate::controls::colour_picker_ctrl::ColourPickerCtrl;
pub use crate::controls::combo_box::{BitmapComboBox, ComboBox};
pub use crate::controls::date_picker_ctrl::{Date, DateFormat, DatePickerCtrl};
pub use crate::controls::dir_picker_ctrl::DirPickerCtrl;
pub use crate::controls::editable_list_box::EditableListBox;
pub use crate::controls::file_ctrl::FileCtrl;
pub use crate::controls::file_picker_ctrl::FilePickerCtrl;
pub use crate::controls::font_picker_ctrl::FontPickerCtrl;
pub use crate::controls::gauge::{Gauge, GaugeStyle};
pub use crate::controls::hyperlink_ctrl::HyperlinkCtrl;
pub use crate::controls::ip_address_ctrl::IPAddressCtrl;
pub use crate::controls::list_box::ListBox;
pub use crate::controls::owner_drawn_combo_box::OwnerDrawnComboBox;
pub use crate::controls::simple_html_list_box::SimpleHtmlListBox;
pub use crate::controls::tree_list_ctrl::{TreeListColumn, TreeListCtrl, TreeListRow};
pub use crate::controls::list_ctrl::{ListCtrl, ListCtrlStyle, ListItem};
pub use crate::controls::radio_box::RadioBox;
pub use crate::controls::search_ctrl::SearchCtrl;
pub use crate::controls::radio_button::RadioButton;
pub use crate::controls::slider::Slider;
pub use crate::controls::spin_button::SpinButton;
pub use crate::controls::spin_ctrl::SpinCtrl;
pub use crate::controls::spin_ctrl_double::SpinCtrlDouble;
pub use crate::controls::static_bitmap::StaticBitmap;
pub use crate::controls::static_box::StaticBox;
pub use crate::controls::static_line::{StaticLine, StaticLineOrientation};
pub use crate::controls::static_text::StaticText;
pub use crate::controls::text_ctrl::TextCtrl;
pub use crate::controls::toggle_button::ToggleButton;
pub use crate::controls::tree_ctrl::{TreeCtrl, TreeItem};
pub use crate::core::accelerator::{Accelerator, Modifiers, ParseError, VirtualKey};
pub use crate::core::accelerator_table::{AcceleratorEntry, AcceleratorTable};
pub use crate::core::frame_events::{
    HelpEvent, IconizeEvent, MaximizeEvent, NavigationKeyEvent, PowerEvent, PowerEventKind,
    UpdateUIEvent,
};
pub use crate::core::char_hook_event::CharHookEvent;
pub use crate::core::control_notify_events::{CheckBoxEvent, ComboBoxEvent, TextEvent};
pub use crate::core::more_control_notify::{
    ButtonEvent, ChoiceEvent, ListBoxEvent, RadioBoxEvent, SearchCtrlEvent, ToggleButtonEvent,
};
pub use crate::core::child_focus_event::ChildFocusEvent;
pub use crate::core::data_view_event::{DataViewEvent, DataViewEventKind};
pub use crate::core::display::Display;
pub use crate::core::display_changed_event::DisplayChangedEvent;
pub use crate::core::module::WxModule;
pub use crate::core::object::WxObject;
pub use crate::core::popup_window_event::{PopupWindowEvent, PopupWindowEventKind};
pub use crate::core::scroll_line_event::ScrollLineEvent;
pub use crate::core::filesystem_watcher_event::{
    FileSystemChangeType, FileSystemWatcherEvent,
};
pub use crate::core::evt_loop_activator::EvtLoopActivator;
pub use crate::core::header_events::{HeaderButtonClickEvent, HeaderColumnEvent};
pub use crate::core::item_container_immutable::ItemContainerImmutable;
pub use crate::core::memory_fs_handler::MemoryFSHandler;
pub use crate::core::progress_event::ProgressEvent;
pub use crate::core::property_grid_event::PropertyGridEvent;
pub use crate::core::wizard_event::{WizardEvent, WizardEventKind};
pub use crate::core::font_enumerator::FontEnumerator;
pub use crate::core::mime_types::{FileTypeInfo, MimeTypesManager};
pub use crate::core::mouse_wheel_event::MouseWheelEvent;
pub use crate::core::palette_events::{PaletteChangedEvent, QueryNewPaletteEvent};
pub use crate::core::timer_event::TimerEvent;
pub use crate::core::file_ctrl_event::FileCtrlEvent;
pub use crate::core::hyperlink_event::HyperlinkEvent;
pub use crate::core::item_container::ItemContainer;
pub use crate::core::notebook_event::NotebookEvent;
pub use crate::core::picker_events::{
    ColourPickerEvent, DatePickerEvent, DirPickerEvent, FilePickerEvent, FontPickerEvent,
};
pub use crate::core::secret_store::SecretStore;
pub use crate::core::sizer_event::SizerEvent;
pub use crate::core::temp_file::TempFile;
pub use crate::core::colour_database::ColourDatabase;
pub use crate::core::container_events::{
    GridEvent, ItemActivateEvent, ListEvent, ListEventKind, TreeEvent, TreeEventKind,
};
pub use crate::core::context_menu_event::ContextMenuEvent;
pub use crate::core::event_loop::EventLoop;
pub use crate::core::drop_files_event::DropFilesEvent;
pub use crate::core::more_events::{
    DpiChangedEvent, FullScreenEvent, JoystickEvent, SetCursorEvent, SysColourChangedEvent,
    UiScrollAxis, UiScrollEvent,
};
pub use crate::core::mouse_events_ext::{
    MouseCaptureLostEvent, MouseEnterEvent, MouseLeaveEvent,
};
pub use crate::core::mouse_state::MouseState;
pub use crate::core::thread_event::ThreadEvent;
pub use crate::core::uri::{Uri, UriError};
pub use crate::core::window_lifecycle_events::{WindowCreateEvent, WindowDestroyEvent};
pub use crate::core::process_event::{ProcessEvent, ProcessEventKind};
pub use crate::core::scroll_win_event::{ScrollWinAxis, ScrollWinEvent};
pub use crate::core::thread_helper::{ThreadHelper, ThreadHelperSimple};
pub use crate::core::app::App;
pub use crate::core::busy_info::BusyInfo;
pub use crate::core::dpi::{
    get_dpi_for_point, get_dpi_for_window, get_process_dpi_awareness, get_system_dpi,
    set_process_dpi_awareness, Dpi, DpiAwareness, SYSTEM_DPI,
};
pub use crate::core::font::{Font, FontDesc};
pub use crate::core::affine_matrix::AffineMatrix2D;
pub use crate::core::appearance::Appearance;
pub use crate::core::caret::Caret;
pub use crate::core::clipboard::Clipboard;
pub use crate::core::config::{
    Config, FileSystemWatcher, Locale, SingleInstanceChecker, StandardPaths,
};
pub use crate::core::context_help_event::ContextHelpEvent;
pub use crate::core::book_ctrl_event::BookCtrlEvent;
pub use crate::core::busy_cursor::BusyCursor;
pub use crate::core::command_event::CommandEvent;
pub use crate::core::cursor::{Cursor, StockCursor};
pub use crate::core::event_handler::{CommandBinder, EvtHandler};
pub use crate::core::file_config::FileConfig;
pub use crate::core::init_dialog_event::InitDialogEvent;
pub use crate::core::reg_config::{RegConfig, RegRoot};
pub use crate::core::window_disabler::WindowDisabler;
pub use crate::core::debug_context::DebugContext;
pub use crate::core::debug_report::{CrashReport, DebugReport, StackWalker};
pub use crate::core::file_system::{FileSystem, FileSystemStream};
pub use crate::core::message_queue::MessageQueue;
pub use crate::core::dir_traverser::{traverse_dir, DirTraverser, FileCollector};
pub use crate::core::menu_event::MenuEvent;
pub use crate::core::process_util::{execute, execute_async, Process};
pub use crate::core::sync_util::{WxCondition, WxCriticalSection, WxSemaphore};
pub use crate::core::thread_util::{SharedFlag, WxMutex, WxThread};
pub use crate::core::close_event::CloseEvent;
pub use crate::core::input_events::{
    FocusEvent, KeyEvent, KeyModifiers, MouseEvent, MouseEventKind, MoveEvent, SizeEvent, SizeType,
};
pub use crate::core::window_events::{
    ActivateEvent, CharEvent, EraseEvent, HideEvent, IdleEvent, NotifyEvent, PaintEvent, ShowEvent,
};
pub use crate::core::geometry::{Colour, Point, Rect, Size};
pub use crate::core::system_settings::SystemSettings;
pub use crate::core::validator::{
    FloatingPointValidator, GenericValidator, IntegerValidator, NonEmptyValidator, RangeValidator,
    Validator,
};
pub use crate::core::timer::Timer;
pub use crate::core::tooltip::ToolTip;
pub use crate::core::widget::{Widget, WidgetRef, Window};
pub use crate::dc::art_provider::{ArtClient, ArtId, ArtProvider};
pub use crate::dc::auto_buffered_paint_dc::AutoBufferedPaintDC;
pub use crate::dc::buffered_dc::BufferedDC;
pub use crate::dc::buffered_paint_dc::BufferedPaintDC;
pub use crate::dc::svg_bitmap::SVGBitmap;
pub use crate::dc::mirror_dc::MirrorDC;
pub use crate::dc::palette::Palette;
pub use crate::dc::bitmap::Bitmap;
pub use crate::dc::bitmap_bundle::{BitmapBundle, RawBitmap};
pub use crate::dc::brush::{Brush, BrushStyle};
pub use crate::dc::dc::{BackgroundMode, ClientDC, Dc, MemoryDC, PaintDC, ScreenDC, WindowDC};
pub use crate::dc::graphics_context::GraphicsContext;
pub use crate::dc::region::Region;
pub use crate::dc::gl_canvas::GLCanvas;
#[cfg(target_os = "windows")]
pub use crate::dc::gl_canvas::gl11;
#[cfg(target_os = "windows")]
pub use crate::dc::icon::svg_bytes_to_hicon;
pub use crate::dc::image::{Image, ImageError, Rgba};
pub use crate::dc::image_list::ImageList;
pub use crate::dc::image_handler::ImageHandler;
pub use crate::dc::bitmap_handler::BitmapHandler;
pub use crate::dc::pen::{Pen, PenStyle};
pub use crate::dialogs::about_dialog::AboutDialog;
pub use crate::dialogs::color_dialog::ColorDialog as ColourDialog;
pub use crate::dialogs::date_picker_dialog::{DateDialogFormat, DatePickerDialog};
pub use crate::dialogs::dir_dialog::DirDialog;
pub use crate::dialogs::file_dialog::{FileDialog, FileDialogStyle};
pub use crate::dialogs::find_replace_dialog::{FindReplaceDialog, FindReplaceEvent};
pub use crate::dialogs::font_dialog::FontDialog;
pub use crate::dialogs::message_box::{message_box, MessageBoxIcon, MessageBoxResult, MessageBoxStyle};
pub use crate::dialogs::message_dialog::{MessageDialog, MessageDialogIcon, MessageDialogStyle};
pub use crate::dialogs::progress_dialog::ProgressDialog;
pub use crate::dialogs::rearrange_dialog::RearrangeDialog;
pub use crate::dialogs::rich_text_formatting_dialog::RichTextFormattingDialog;
pub use crate::dialogs::credential_entry_dialog::CredentialEntryDialog;
pub use crate::dialogs::property_sheet_dialog::{PropertySheetDialog, PropertySheetDialogResult};
pub use crate::dialogs::single_choice_dialog::{ChoiceResult, MultiChoiceDialog, SingleChoiceDialog};
pub use crate::dialogs::symbol_picker_dialog::SymbolPickerDialog;
pub use crate::dialogs::text_entry_dialog::{NumberEntryDialog, PasswordEntryDialog, TextEntryDialog};
pub use crate::dnd::drag_image::DragImage;
pub use crate::dnd::drop_target::DroppedFiles;
pub use crate::io::{
    BufferedInputStream, BufferedOutputStream, FFileInputStream, FFileOutputStream, FileInputStream,
    FileOffset,
    FileOutputStream,
    FilterInputStream, FilterOutputStream, MemoryInputStream, MemoryOutputStream, StreamBuffer,
    TextInputStream, TextOutputStream, WxFFile, WxInputStream, WxOutputStream, ZlibInputStream,
    ZlibOutputStream, CountingOutputStream, CountingInputStream, TeeInputStream, InputStreamExt,
    OutputStreamExt, StreamBase,
    StreamError,
};
pub use crate::core::array_string::ArrayString;
pub use crate::core::archive_fs_handler::ArchiveFSHandler;
pub use crate::core::cmdline_parser::CmdLineParser;
pub use crate::core::datetime::DateTime;
pub use crate::core::datetime_span::{DateSpan, TimeSpan};
pub use crate::core::platform_info::{Arch, OsFamily, PlatformInfo};
pub use crate::core::string_tokenizer::StringTokenizer;
pub use crate::core::version_info::VersionInfo;
pub use crate::core::zip_fs_handler::ZipFSHandler;
pub use crate::core::dynamic_library::DynamicLibrary;
pub use crate::core::environment::Environment;
pub use crate::core::file_name::FileName;
pub use crate::core::internet_fs_handler::InternetFSHandler;
pub use crate::core::long_long::LongLong;
pub use crate::core::ulong_long::ULongLong;
pub use crate::core::regex::RegEx;
pub use crate::core::text_buffer::TextBuffer;
pub use crate::core::variant::Variant;
pub use crate::core::class_info::ClassInfo;
pub use crate::core::ref_counter::RefCounter;
pub use crate::core::weak_ref::WeakRef;
pub use crate::core::window_update_locker::WindowUpdateLocker;
pub use crate::core::event_filter::{BlockListFilter, EventFilter, PassThroughFilter};
pub use crate::core::translation::{get_translation, Translation};
pub use crate::core::scoped_ptr::ScopedPtr;
pub use crate::core::wx_any::WxAny;
pub use crate::core::client_data::{ClientData, ObjectClientData, StringClientData};
pub use crate::core::array_int::ArrayInt;
pub use crate::core::array_long::ArrayLong;
pub use crate::core::array_double::ArrayDouble;
pub use crate::core::string_list::StringList;
pub use crate::core::geometry2d::{Point2D, Rect2D, Size2D};
pub use crate::core::kill_focus_event::KillFocusEvent;
pub use crate::core::set_focus_event::SetFocusEvent;
pub use crate::core::nc_paint_event::NcPaintEvent;
pub use crate::core::sys_command_event::SysCommandEvent;
pub use crate::core::activate_app_event::ActivateAppEvent;
pub use crate::core::process_exit_event::ProcessExitEvent;
pub use crate::core::object_ref_data::ObjectRefData;
pub use crate::core::hash_set::WxHashSet;
pub use crate::core::hash_map::WxHashMap;
pub use crate::core::nc_calc_size_event::NcCalcSizeEvent;
pub use crate::core::mouse_capture_changed_event::MouseCaptureChangedEvent;
pub use crate::core::zip_entry::ZipEntry;
pub use crate::core::archive_entry::ArchiveEntry;
pub use crate::core::tar_entry::TarEntry;
pub use crate::core::nc_hit_test_event::NcHitTestEvent;
pub use crate::core::query_layout_event::QueryLayoutEvent;
pub use crate::core::calculate_layout_event::CalculateLayoutEvent;
pub use crate::core::path_env::PathEnv;
pub use crate::core::path_list::PathList;
pub use crate::core::sorted_array_string::SortedArrayString;
pub use crate::core::temp_dir::TempDir;
pub use crate::core::text_file::TextFile;
pub use crate::core::wx_dir::WxDir;
pub use crate::core::wx_file::WxFile;
pub use crate::net::{
    FtpClient, HttpClient, IpcClient, IpcConnection, IpcServer, Protocol, Socket, SocketEvent,
    SocketEventKind, SocketServer, Url, WebRequest,
};
pub use crate::printing::postscript_dc::PostScriptDC;
pub use crate::printing::preview_control_bar::PreviewControlBar;
pub use crate::printing::preview_frame::PreviewFrame;
pub use crate::printing::printer_dc::PrinterDC;
pub use crate::printing::{PageSetupDialog, PrintDialog, Printer, Printout, PrintPreview};
pub use crate::platform::stub_backend::StubBackend;
pub use crate::platform::appkit_stubs::{
    AppKitApp, AppKitButton, AppKitFrame, AppKitPanel, AppKitStaticText,
};
pub use crate::platform::gtk_stubs::{GtkApp, GtkButton, GtkFrame, GtkPanel, GtkStaticText};
pub use crate::dnd::ole_dnd::{OleDropEffect, OleDropError, OleDroppedData, OleDropPosition};
#[cfg(target_os = "windows")]
pub use crate::dnd::ole_dnd::{
    DragContinueResult, OleDragData, OleDragError, OleDragSource, OleDragSourceCallbacks,
    OleDropTarget,
};
pub use crate::window::banner_window::BannerWindow;
pub use crate::window::file_history::FileHistory;
pub use crate::window::popup_transient_window::PopupTransientWindow;
pub use crate::window::rich_tooltip::RichToolTip;
pub use crate::window::dialog::Dialog;
pub use crate::window::frame::{Frame, FrameBuilder};
pub use crate::window::frame_extras::{MiniFrame, SplashScreen, TipWindow};
pub use crate::window::mdi::{MDIChildFrame, MDIParentFrame};
pub use crate::window::menu::{Menu, MenuBar, MenuItem, MenuItemKind};
pub use crate::window::native_window::NativeWindow;
pub use crate::window::panel::Panel;
pub use crate::window::popup_menu::PopupMenu;
pub use crate::window::popup_window::PopupWindow;
pub use crate::window::layer_window::LayerWindow;
pub use crate::window::dwm_style::BackdropType;
pub use crate::window::top_level_window::{
    CentreDirection, FullScreenStyle, TopLevelWindow, UserAttentionFlags, WindowCornerPreference,
};

/// Convenient re-exports of the most commonly used items.
///
/// See [`prelude`](self) for details.
pub mod prelude;
