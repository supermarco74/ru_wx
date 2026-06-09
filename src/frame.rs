//! Top-level window (`wxFrame`).
//!
//! `Frame` is the only top-level window type in `ru_wx`. On Windows it
//! owns a single hidden `HWND` plus a `Rc<RefCell<FrameData>>` that
//! lives in `GWLP_USERDATA` and is reachable from the window procedure.
//! The frame:
//!
//! * owns the per-frame `BoxSizer` used for automatic layout,
//! * dispatches `WM_COMMAND` messages (button clicks, menu selections)
//!   to user-registered handlers keyed by control id,
//! * dispatches `WM_NOTIFY` messages (e.g. the `Tab` control's selection
//!   change) the same way,
//! * dispatches user-defined `WM_APP + n` messages used by `IconTray`,
//! * paints its own background colour in response to `WM_ERASEBKGND`.
//!
//! Construct a frame through [`Frame::builder`]:
//!
//! ```no_run
//! use ru_wx::prelude::*;
//! let frame = Frame::builder()
//!     .with_title("My app")
//!     .with_size(800, 600)
//!     .build();
//! ```

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::accelerator::Accelerator;
use crate::dpi::{get_dpi_for_point, get_dpi_for_window, get_system_dpi, Dpi};
use crate::drop_target::{self, DroppedFiles};
use crate::ole_dnd::{self, OleDropError, OleDropTarget, OleDroppedData, OleDropPosition};
use crate::geometry::Colour;
use crate::menu::MenuBar;
use crate::sizer::BoxSizer;
use crate::widget::{WidgetRef, Window};

#[cfg(target_os = "windows")]
use crate::platform::win32::to_wide;
#[cfg(target_os = "windows")]
use windows_sys::Win32::Foundation::*;
#[cfg(target_os = "windows")]
use windows_sys::Win32::Graphics::Gdi::*;
#[cfg(target_os = "windows")]
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::Controls::NMHDR;
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::Shell::{DragAcceptFiles, HDROP};
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::WindowsAndMessaging::*;

/// Shared frame data accessible from WndProc
pub(crate) struct FrameData {
    pub hwnd: HWND,
    pub widgets: Vec<WidgetRef>,
    pub command_handlers: HashMap<u16, Box<dyn FnMut()>>,
    /// Handlers invoked when a child control receives a WM_NOTIFY message
    /// (used by the `Tab`, `TreeCtrl`, and `ListCtrl` controls, which
    /// notify via `WM_NOTIFY` rather than `WM_COMMAND`). Keyed by the
    /// control's `idFrom`. The handler receives the NMHDR `code` field
    /// as its only argument so it can filter by notification type (e.g.
    /// `TVN_SELCHANGED` vs. `TVN_SELCHANGING`).
    pub notify_handlers: HashMap<u16, Box<dyn FnMut(u32)>>,
    /// Handlers invoked when a child control in `LVS_OWNERDATA`
    /// (virtual) ListView mode dispatches a `LVN_GETDISPINFOW`
    /// notification. Keyed by the control's `idFrom`. The handler
    /// receives the full `lparam` (a pointer to a
    /// `tagNMLVDISPINFOW` from `<commctrl.h>`) cast to `isize`,
    /// which is what the `LVN_GETDISPINFOW` handler re-interprets
    /// back to a `*mut NMLVDISPINFOW` to read the request fields
    /// and write the response string.
    ///
    /// This map is separate from `notify_handlers` because the
    /// disp-info callback needs the full `lparam` (the notification
    /// body is the data the callback writes back, not just a code
    /// to filter on), so the simpler `Box<dyn FnMut(u32)>` signature
    /// of the regular `notify_handlers` is not enough. The two maps
    /// are independent and can both be populated for the same
    /// `idFrom` (e.g. a virtual list that also wants
    /// `LVN_ITEMCHANGED`).
    pub disp_info_handlers: HashMap<u16, Box<dyn FnMut(isize)>>,
    /// Handlers invoked when a child `SysDateTimePick32` control
    /// dispatches a `DTN_DATETIMECHANGE` notification (i.e. the
    /// user picked a different date, or cleared the control
    /// with `DTS_SHOWNONE`). Keyed by the control's `idFrom`.
    /// The handler receives the full `lparam` (a pointer to a
    /// `tagNMDATETIMECHANGE` from `<commctrl.h>`), which the
    /// registered handler re-interprets back to a `*mut
    /// NMDATETIMECHANGE` to read the new `SYSTEMTIME` value (and
    /// the `GDT_VALID` / `GDT_NONE` flag).
    ///
    /// This map is separate from `notify_handlers` because the
    /// date-change callback needs the full `lparam` (the
    /// notification body is the data the callback reads, not
    /// just a code to filter on), so the simpler
    /// `Box<dyn FnMut(u32)>` signature of the regular
    /// `notify_handlers` is not enough. The two maps are
    /// independent and can both be populated for the same
    /// `idFrom` (e.g. a date picker that also wants
    /// `NM_KILLFOCUS`).
    pub dtn_handlers: HashMap<u16, Box<dyn FnMut(isize)>>,
    /// Handlers invoked when the frame receives a user-defined message
    /// in the `WM_APP + n` range (used by the `IconTray` for shell
    /// notification area callback messages). Keyed by the message id.
    /// The handler receives the message's `lparam` (the mouse / NIN_*
    /// event code) as its only argument.
    pub tray_message_handlers: HashMap<u32, Box<dyn FnMut(u32)>>,
    /// Handlers invoked when a child `SCROLLBAR` control dispatches
    /// a `WM_HSCROLL` or `WM_VSCROLL` notification to the parent
    /// (SB_CTL scroll bars are not subclassed - Win32 routes those
    /// messages to the parent frame, not the control itself).
    /// Keyed by the scroll bar's `HWND`. The handler receives the
    /// low word of `wparam` (the SB_* request code, e.g.
    /// `SB_LINEUP`, `SB_PAGEDOWN`, `SB_THUMBPOSITION`) as its
    /// first argument and the high word of `wparam` (the thumb
    /// position for `SB_THUMBPOSITION` / `SB_THUMBTRACK`, or 0
    /// for other codes) as its second. The generic signature
    /// keeps `frame.rs` independent of `scroll_bar.rs`'s typed
    /// `ScrollEvent` enum - the `scroll_bar` module wraps its
    /// user callback into this signature in `on_scroll`.
    pub scroll_handlers: HashMap<HWND, Box<dyn FnMut(u16, i32)>>,
    /// Handlers invoked when the frame receives a `WM_PAINT`
    /// message (i.e. the OS is asking the window to repaint its
    /// client area). The handler receives the Win32 `HDC` of the
    /// `BeginPaint` call as its only argument, so it can draw
    /// straight into the frame's window DC. The frame's
    /// `WM_PAINT` arm wraps the dispatch in the standard
    /// `BeginPaint` / `EndPaint` pair so the handler is free to
    /// call any GDI drawing primitive.
    ///
    /// `None` in the slot means "no paint handler is registered",
    /// in which case the frame falls back to `DefWindowProcW`
    /// (the default background colour / sizer-driven children
    /// still draw themselves normally). Multiple paint handlers
    /// can be registered - they fire in registration order, the
    /// same as the resize callback list.
    pub paint_handlers: Vec<Box<dyn FnMut(isize)>>,
    /// Keyboard accelerators registered for this frame. Each entry is
    /// `(Accelerator, command_id)` where `command_id` is the same id
    /// passed to [`Frame::register_command_handler`]. The frame's
    /// message loop calls `TranslateAcceleratorW` with an `HACCEL`
    /// table built from this list, so the matching `command_handler`
    /// fires when the user presses the binding, even when the owning
    /// menu is hidden.
    pub accelerators: Vec<(Accelerator, u16)>,
    /// The menu bar currently attached to the frame, if any.
    /// Stored so that [`Frame::replace_accelerator`],
    /// [`Frame::unregister_accelerator`] and
    /// [`Frame::clear_accelerators`] can refresh the Win32 menu
    /// labels (via [`crate::menu::MenuBar::update_item_shortcut`])
    /// to keep the visible shortcut hint in sync with the in-memory
    /// accelerator table.
    pub menu_bar: Option<MenuBar>,
    pub sizer: Option<BoxSizer>,
    pub background_colour: Colour,
    pub on_resize: Vec<Box<dyn FnMut(u32, u32)>>,
    pub on_close: Option<Box<dyn FnMut()>>,
    /// Handler invoked when the frame receives a `WM_DROPFILES`
    /// message (i.e. the user dropped one or more files from
    /// Windows Explorer onto the frame's window). The Shell-level
    /// protocol used here is `DragAcceptFiles` / `DragQueryFileW`,
    /// which only carries file paths — not text, not in-memory
    /// data objects. The OLE COM drop-target protocol is a
    /// separate, larger surface and is not implemented yet.
    ///
    /// `None` means "no drop handler is registered": in that case
    /// the frame will not even call `DragAcceptFiles(TRUE)` during
    /// `build`, so the Shell will not deliver `WM_DROPFILES`
    /// messages to it at all (the default Windows behaviour is to
    /// disable drops).
    pub drop_files_handler: Option<Box<dyn FnMut(DroppedFiles)>>,
    /// The OLE COM `IDropTarget` registered with this frame's
    /// window, if any. Owned by the frame so that the COM
    /// drop-target lifetime matches the window's lifetime —
    /// the `OleDropTarget`'s `Drop` impl calls `RevokeDragDrop`
    /// and releases the IUnknown refcount, so the COM runtime
    /// is fully torn down when the frame is dropped.
    ///
    /// `None` means "no OLE drop target is registered" — in
    /// that case `RegisterDragDrop` has not been called, and
    /// the OLE runtime will not deliver `IDropTarget::*` calls
    /// to this window. (`WM_DROPFILES` from the Shell-level
    /// protocol is independent and is governed by
    /// `drop_files_handler` above.)
    pub ole_drop_target: Option<OleDropTarget>,
}

// CREATING_FRAME thread-local removed: the Rc<RefCell<FrameData>> pointer
// is stored via SetWindowLongPtrW immediately after CreateWindowExW returns,
// so no messages during creation need access to frame data.

#[derive(Clone)]
pub struct Frame {
    pub(crate) inner: Rc<RefCell<FrameData>>,
}

pub struct FrameBuilder {
    title: String,
    width: u32,
    height: u32,
    x: i32,
    y: i32,
}

impl Frame {
    pub fn builder() -> FrameBuilder {
        FrameBuilder {
            title: String::from("ru_wx Window"),
            width: 800,
            height: 600,
            x: CW_USEDEFAULT,
            y: CW_USEDEFAULT,
        }
    }

    /// Get the native window handle
    pub fn hwnd(&self) -> HWND {
        self.inner.borrow().hwnd
    }

    /// The DPI value of the monitor that currently hosts this
    /// frame. Changes as the frame is dragged between monitors
    /// with different DPI settings (the library's manifest
    /// declares `PerMonitorV2` awareness, which makes the OS
    /// re-emit `WM_DPICHANGED` on every move).
    ///
    /// Returns the system DPI (96) if the frame has not yet
    /// been shown (i.e. the underlying `HWND` is null).
    #[cfg(target_os = "windows")]
    pub fn dpi(&self) -> Dpi {
        let hwnd = self.inner.borrow().hwnd;
        get_dpi_for_window(hwnd)
    }

    /// The scale factor relative to the 96-DPI baseline
    /// (1.0 at 100%, 1.5 at 150%, 2.0 at 200%, 3.0 at 300%).
    ///
    /// Convenience wrapper around [`Frame::dpi`]; useful when
    /// the user only needs the multiplier, not the raw DPI
    /// value.
    #[cfg(target_os = "windows")]
    pub fn scale_factor(&self) -> f32 {
        self.dpi().scale_factor()
    }

    /// Add a widget (any type implementing Widget trait)
    pub fn add_widget(&self, widget: WidgetRef) {
        self.inner.borrow_mut().widgets.push(widget);
    }

    /// Register a command handler (for button clicks, menu items)
    pub fn register_command_handler(&self, id: u16, handler: Box<dyn FnMut()>) {
        self.inner.borrow_mut().command_handlers.insert(id, handler);
    }

    /// Register a notify handler (for controls that use `WM_NOTIFY`,
    /// such as the `Tab` control). The handler is keyed by the control's
    /// `idFrom` and is invoked whenever the frame receives a `WM_NOTIFY`
    /// message whose `idFrom` matches. The handler receives the NMHDR
    /// `code` field as its only argument so it can filter by notification
    /// type (e.g. `TVN_SELCHANGED` vs. `TVN_SELCHANGING`).
    pub fn register_notify_handler(&self, id: u16, handler: Box<dyn FnMut(u32)>) {
        self.inner.borrow_mut().notify_handlers.insert(id, handler);
    }

    /// Register a disp-info handler for a child `ListCtrl` in
    /// `LVS_OWNERDATA` (virtual) mode. The handler is keyed by the
    /// control's `idFrom` and is invoked whenever the frame receives
    /// a `LVN_GETDISPINFOW` notification whose `idFrom` matches.
    ///
    /// The handler receives the full `lparam` of the `WM_NOTIFY`
    /// message — that is, a pointer to a `tagNMLVDISPINFOW` from
    /// `<commctrl.h>`, cast to `isize`. The list-view uses this
    /// pointer both to read the request (which cell, which
    /// sub-item, which mask bits) and to write the response
    /// (the UTF-16 string the callback puts in the
    /// `item.pszText` buffer).
    ///
    /// This is a separate registration path from
    /// [`Frame::register_notify_handler`] because the regular
    /// notify handler's `Box<dyn FnMut(u32)>` signature only
    /// carries the NMHDR `code`; for `LVN_GETDISPINFOW` the
    /// entire notification body **is** the data the callback
    /// writes back, so we need the pointer. Both maps can be
    /// populated for the same `idFrom` independently
    /// (e.g. a virtual list that also wants
    /// `LVN_ITEMCHANGED`).
    pub fn register_disp_info_handler(&self, id: u16, handler: Box<dyn FnMut(isize)>) {
        self.inner
            .borrow_mut()
            .disp_info_handlers
            .insert(id, handler);
    }

    /// Register a date-time change handler for a child
    /// `SysDateTimePick32` (i.e. a [`crate::DatePickerCtrl`]). The
    /// handler is keyed by the control's `idFrom` and is invoked
    /// whenever the frame receives a `DTN_DATETIMECHANGE`
    /// notification whose `idFrom` matches. The
    /// [`crate::DatePickerCtrl::on_date_change`] method uses this
    /// registration path internally.
    ///
    /// The handler receives the full `lparam` of the `WM_NOTIFY`
    /// message — that is, a pointer to a `tagNMDATETIMECHANGE`
    /// from `<commctrl.h>`, cast to `isize`. The handler reads
    /// the `dwFlags` field (which is `GDT_VALID` if the new date
    /// is valid, or `GDT_NONE` if the control was cleared with
    /// `DTS_SHOWNONE`) and the new `SYSTEMTIME` from the `st`
    /// field. This is a separate registration path from
    /// [`Frame::register_notify_handler`] because the regular
    /// notify handler's `Box<dyn FnMut(u32)>` signature only
    /// carries the NMHDR `code`; for `DTN_DATETIMECHANGE` the
    /// entire notification body **is** the data the callback
    /// reads, so we need the pointer. Both maps can be
    /// populated for the same `idFrom` independently.
    pub fn register_dtn_handler(&self, id: u16, handler: Box<dyn FnMut(isize)>) {
        self.inner.borrow_mut().dtn_handlers.insert(id, handler);
    }

    /// Register a handler for a user-defined message in the `WM_APP + n`
    /// range. Used by `IconTray` to receive shell notification area
    /// callback messages. The handler is invoked with the message's
    /// `lparam` (the mouse / `NIN_*` event code) as its only argument.
    pub fn register_tray_message_handler(&self, msg: u32, handler: Box<dyn FnMut(u32)>) {
        self.inner
            .borrow_mut()
            .tray_message_handlers
            .insert(msg, handler);
    }

    /// Remove a previously-registered tray message handler. Called by
    /// `IconTray::drop` so the closure doesn't outlive the tray.
    pub fn unregister_tray_message_handler(&self, msg: u32) {
        self.inner.borrow_mut().tray_message_handlers.remove(&msg);
    }

    /// Register a handler for a child `SCROLLBAR` control. Win32 routes
    /// `WM_HSCROLL` and `WM_VSCROLL` notifications from SB_CTL scroll bars
    /// to the parent frame (not to the control itself), so the dispatch
    /// table lives on the frame and is keyed by the scroll bar's `HWND`.
    ///
    /// `handler` receives `(code, position)` where:
    /// * `code` is the low word of `wparam` — the SB_* request code
    ///   (`SB_LINEUP`, `SB_LINEDOWN`, `SB_PAGEUP`, `SB_PAGEDOWN`,
    ///   `SB_THUMBPOSITION`, `SB_THUMBTRACK`, `SB_TOP`, `SB_BOTTOM`,
    ///   `SB_ENDSCROLL`).
    /// * `position` is the high word of `wparam` — the thumb position
    ///   for `SB_THUMBPOSITION` / `SB_THUMBTRACK`, or 0 for the other
    ///   request codes.
    ///
    /// The signature is kept generic (no `ScrollEvent` enum) so
    /// `frame.rs` does not depend on the `scroll_bar` module. The
    /// `ScrollBar::on_scroll` wrapper does the conversion.
    pub fn register_scroll_handler<F>(&self, hwnd: HWND, handler: F)
    where
        F: FnMut(u16, i32) + 'static,
    {
        self.inner
            .borrow_mut()
            .scroll_handlers
            .insert(hwnd, Box::new(handler));
    }

    /// Remove a previously-registered scroll handler. Called by
    /// `ScrollBar::drop` so the closure doesn't outlive the control.
    pub fn unregister_scroll_handler(&self, hwnd: HWND) {
        self.inner.borrow_mut().scroll_handlers.remove(&hwnd);
    }

    /// Register a handler for the frame's `WM_PAINT` message. The
    /// handler is invoked once per paint cycle, inside the standard
    /// `BeginPaint` / `EndPaint` pair, with the `HDC` of the
    /// `BeginPaint` call cast to `isize`. The handler is free to
    /// call any GDI drawing primitive on that DC (the actual
    /// `DC` wrappers — `PaintDC`, `WindowDC`, `MemoryDC` — wrap
    /// the same Win32 primitives and can be used inside the
    /// callback).
    ///
    /// Multiple paint handlers can be registered; they fire in
    /// registration order, the same as the resize callback list.
    /// If no paint handler is registered the frame falls back to
    /// `DefWindowProcW` (the default background colour and any
    /// sizer-driven child widgets still draw themselves).
    pub fn register_paint_handler<F>(&self, handler: F)
    where
        F: FnMut(isize) + 'static,
    {
        self.inner.borrow_mut().paint_handlers.push(Box::new(handler));
    }

    /// Register a keyboard accelerator (`Ctrl+S`, `F5`, `Alt+F4`, …) that
    /// will fire the command handler registered under `command_id` when
    /// the user presses the binding, even when the owning menu is hidden.
    ///
    /// Accelerators registered after [`Frame::show`] has started the
    /// message loop are not picked up automatically - they apply to the
    /// accelerator table built at loop entry. The intended pattern is to
    /// register all accelerators during the menu / widget construction
    /// phase (i.e. after `Frame::builder().build()` and before
    /// `frame.show()`).
    ///
    /// Duplicate bindings (same `Accelerator` registered more than once)
    /// are accepted; the first matching `command_id` wins, mirroring the
    /// Win32 `HACCEL` lookup order.
    pub fn register_accelerator(&self, accel: Accelerator, command_id: u16) {
        self.inner
            .borrow_mut()
            .accelerators
            .push((accel, command_id));
    }

    /// Test-only constructor that builds a `Frame` whose `HWND` is
    /// `null` (i.e. the frame has not been "created" through the
    /// Win32 `CreateWindowExW` path). It exists so that unit tests in
    /// this module — and any future `tests/*.rs` integration suite
    /// that adds a `pub mod testing` re-export — can exercise the
    /// platform-agnostic parts of the public surface (accelerator
    /// registration, command-handler dispatch table, sizer storage)
    /// without having to spin up a real Win32 message loop.
    ///
    /// The returned frame is **not** functional as a window — the
    /// `HWND` is `null` and any method that ultimately touches the
    /// window handle (e.g. `set_size`, `close`, `dpi`) is only safe
    /// to call for the specific null-handle cases the public API
    /// already supports.
    #[cfg(test)]
    #[allow(dead_code)]
    pub(crate) fn for_testing() -> Frame {
        let data = FrameData {
            hwnd: std::ptr::null_mut(),
            widgets: Vec::new(),
            command_handlers: HashMap::new(),
            notify_handlers: HashMap::new(),
            disp_info_handlers: HashMap::new(),
            dtn_handlers: HashMap::new(),
            tray_message_handlers: HashMap::new(),
            scroll_handlers: HashMap::new(),
            paint_handlers: Vec::new(),
            accelerators: Vec::new(),
            menu_bar: None,
            sizer: None,
            background_colour: Colour::LIGHT_GREY,
            on_resize: Vec::new(),
            on_close: None,
            drop_files_handler: None,
            ole_drop_target: None,
        };
        Frame {
            inner: Rc::new(RefCell::new(data)),
        }
    }

    /// Iterate over the registered accelerators, in registration order.
    /// Useful for diagnostic UIs or for re-binding a single accelerator
    /// at runtime.
    pub fn accelerators(&self) -> Vec<(Accelerator, u16)> {
        self.inner.borrow().accelerators.clone()
    }

    /// Remove the first registered entry that matches `accel`. The
    /// remaining entries keep their relative order (the list is
    /// re-indexed after the removal but no other entry moves).
    ///
    /// Returns `true` if a matching entry was removed, `false` if no
    /// such binding was registered. Duplicate bindings are tolerated
    /// by [`Frame::register_accelerator`] but only one is removed per
    /// call: callers who want to clear every occurrence should call
    /// this method repeatedly or [`Frame::clear_accelerators`] for a
    /// bulk reset.
    ///
    /// If a menu bar is attached, the corresponding menu item's
    /// shortcut is cleared in lockstep so the visible Win32 label
    /// matches the in-memory state.
    ///
    /// Note: like [`Frame::register_accelerator`], this only mutates
    /// the in-memory list. The accelerator table actually fed to
    /// `TranslateAcceleratorW` is built from this list once, at the
    /// start of the message loop, so a change made after the loop has
    /// started will not take effect on the running session.
    pub fn unregister_accelerator(&self, accel: Accelerator) -> bool {
        let mut data = self.inner.borrow_mut();
        if let Some(pos) = data.accelerators.iter().position(|(a, _)| *a == accel) {
            let (_, id) = data.accelerators.remove(pos);
            if let Some(menubar) = data.menu_bar.as_mut() {
                menubar.update_item_shortcut(id, None);
            }
            true
        } else {
            false
        }
    }

    /// Remove all registered accelerator bindings. The frame ends up
    /// in the same state as a freshly-built frame with respect to
    /// `accelerators()`. Calling this on an already-empty list is a
    /// no-op.
    ///
    /// If a menu bar is attached, every menu item with a shortcut is
    /// reset to `None` in lockstep so the visible Win32 labels
    /// match the in-memory state.
    ///
    /// Like the other mutators on this list, this only affects the
    /// in-memory table; the `HACCEL` actually in use by the running
    /// message loop is not rebuilt automatically.
    pub fn clear_accelerators(&self) {
        let mut data = self.inner.borrow_mut();
        let ids: Vec<u16> = data.accelerators.iter().map(|(_, id)| *id).collect();
        data.accelerators.clear();
        if let Some(menubar) = data.menu_bar.as_mut() {
            for id in ids {
                menubar.update_item_shortcut(id, None);
            }
        }
    }

    /// Atomically rebind `old` to `new` in a single step. If an entry
    /// for `old` is found, the entry at the same position is replaced
    /// by `(new, command_id)` and `true` is returned. If no entry for
    /// `old` exists the list is left unchanged and `false` is
    /// returned.
    ///
    /// Other entries keep their relative order. This is the operation
    /// to call when the user changes a shortcut at runtime (e.g. an
    /// "Options" dialog lets them pick a new `Ctrl+S` binding): one
    /// call replaces the old binding without leaving a stale entry
    /// behind.
    ///
    /// If a menu bar is attached, the menu item identified by
    /// `command_id` has its shortcut rewritten in lockstep so the
    /// visible Win32 label matches the new binding.
    ///
    /// If `new` is already bound to a different command, both entries
    /// will coexist after the rebind; the first matching `command_id`
    /// wins in the Win32 `HACCEL` lookup. This mirrors the duplicate
    /// tolerance of [`Frame::register_accelerator`] — if strict
    /// dedup is required, call [`Frame::unregister_accelerator`] on
    /// `new` first.
    pub fn replace_accelerator(&self, old: Accelerator, new: Accelerator, command_id: u16) -> bool {
        let mut data = self.inner.borrow_mut();
        if let Some(slot) = data.accelerators.iter_mut().find(|(a, _)| *a == old) {
            *slot = (new, command_id);
            if let Some(menubar) = data.menu_bar.as_mut() {
                menubar.update_item_shortcut(command_id, Some(new));
            }
            true
        } else {
            false
        }
    }

    /// Set a BoxSizer for automatic layout
    pub fn set_sizer(&self, sizer: BoxSizer) {
        self.inner.borrow_mut().sizer = Some(sizer);
        self.do_layout();
    }

    /// Set the menu bar
    ///
    /// Takes the `MenuBar` by value (rather than by reference) so
    /// the frame can keep an owned copy. The copy is needed for the
    /// [`Frame::replace_accelerator`], [`Frame::unregister_accelerator`]
    /// and [`Frame::clear_accelerators`] mutators, which walk the
    /// menu and refresh each item's Win32 label so the visible
    /// shortcut hint stays in sync with the in-memory accelerator
    /// table.
    ///
    /// Calling this method on a frame that already has a menu bar
    /// drops the previous bar (the Win32 `HMENU` is detached and
    /// the previous `MenuBar` is freed through its `Drop` impl).
    pub fn set_menu_bar(&self, menubar: MenuBar) {
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            let hwnd = self.inner.borrow().hwnd;
            SetMenu(hwnd, menubar.hmenu());
            DrawMenuBar(hwnd);
            let _ = hwnd; // borrow dropped here
        }
        self.inner.borrow_mut().menu_bar = Some(menubar);
    }

    /// Set background colour
    pub fn set_background_colour(&self, colour: Colour) {
        self.inner.borrow_mut().background_colour = colour;
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            let hwnd = self.inner.borrow().hwnd;
            InvalidateRect(hwnd, std::ptr::null(), 1);
            let _ = hwnd; // borrow dropped here
        }
    }

    /// Register a callback that fires when the frame is resized.
    ///
    /// Multiple callbacks can be registered on the same frame: each one
    /// is invoked in registration order on every `WM_SIZE`. This is the
    /// public API used by user code; internal widgets (e.g. `StatusBar`)
    /// call the crate-internal `add_resize_handler` counterpart instead.
    pub fn on_resize<F: FnMut(u32, u32) + 'static>(&self, f: F) {
        self.inner.borrow_mut().on_resize.push(Box::new(f));
    }

    /// Internal-only counterpart of [`Frame::on_resize`] used by widgets
    /// that need to react to the frame's resize (currently `StatusBar`).
    /// Kept `pub(crate)` so user code uses the public `on_resize` setter
    /// and so the surface area visible from outside the crate stays
    /// minimal.
    pub(crate) fn add_resize_handler<F: FnMut(u32, u32) + 'static>(&self, f: F) {
        self.inner.borrow_mut().on_resize.push(Box::new(f));
    }

    /// Set on_close callback
    pub fn on_close<F: FnMut() + 'static>(&self, f: F) {
        self.inner.borrow_mut().on_close = Some(Box::new(f));
    }

    /// Register a callback that fires when one or more files are
    /// dropped onto the frame's window from Windows Explorer (or
    /// any other Shell source that uses the `WM_DROPFILES` message).
    ///
    /// The callback receives a [`DroppedFiles`] value containing the
    /// absolute file paths that were dropped, in the order the Shell
    /// delivered them. If the user drops nothing — e.g. the drag
    /// target rejects the operation before `WM_DROPFILES` is
    /// emitted — the callback is simply not called.
    ///
    /// # Cross-platform behaviour
    ///
    /// This method is available on every platform (it just stores
    /// the callback in `FrameData`). The callback is only ever
    /// invoked on Windows: the Shell-level drag-and-drop protocol
    /// is Windows-specific. On macOS / Linux the callback is
    /// silently ignored. The full OLE COM drag-and-drop protocol
    /// (`IDropTarget` / `IDataObject`) is a separate, larger
    /// surface and is **not** yet implemented in `ru_wx`.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ru_wx::prelude::*;
    /// use ru_wx::DroppedFiles;
    ///
    /// let frame = Frame::builder().with_title("Files").build();
    /// frame.set_drop_files_callback(|files: DroppedFiles| {
    ///     for path in files.paths() {
    ///         println!("dropped: {}", path.display());
    ///     }
    /// });
    /// ```
    ///
    /// # Replacement semantics
    ///
    /// Calling this method again replaces the previous callback
    /// (the old `Box<dyn FnMut>` is dropped). There is no
    /// "register multiple callbacks" or "chain handlers" support
    /// — drop handling is one-shot by design, because the typical
    /// use case is "open these files" and that has a single
    /// natural owner.
    pub fn set_drop_files_callback<F: FnMut(DroppedFiles) + 'static>(&self, f: F) {
        self.inner.borrow_mut().drop_files_handler = Some(Box::new(f));
    }

    /// Register an OLE COM drag-and-drop callback on this
    /// frame.
    ///
    /// This is the **destination-side** of OLE COM
    /// drag-and-drop, complementing the Shell-level
    /// [`set_drop_files_callback`](Self::set_drop_files_callback)
    /// (v0.5.5). Where `set_drop_files_callback` only sees
    /// file drops from Windows Explorer (the Shell-level
    /// `WM_DROPFILES` protocol), the OLE COM path sees
    /// *any* drop that produces an `IDataObject`: files from
    /// Explorer (Shell also produces an `IDataObject` in
    /// addition to the `WM_DROPFILES` message), text from
    /// Notepad / browsers / `Edit` / `RichEdit` controls,
    /// and in the future in-app data (once the source-side
    /// `DoDragDrop` wrapper lands).
    ///
    /// The two protocols **coexist**: a frame can have a
    /// Shell-level handler and an OLE handler registered at
    /// the same time, and the Shell / COM will each deliver
    /// their preferred format. User code that wants to be
    /// tolerant of both should register both callbacks.
    ///
    /// # Format priority
    ///
    /// The OLE COM `IDataObject` is queried for formats in
    /// this order:
    ///
    /// 1. `CF_HDROP` (file paths; Explorer always offers
    ///    this) — [`crate::OleDroppedData::Files`].
    /// 2. `CF_UNICODETEXT` (UTF-16 text) —
    ///    [`crate::OleDroppedData::Text`].
    /// 3. Anything else — [`crate::OleDroppedData::Other`].
    ///
    /// # Cross-platform behaviour
    ///
    /// The method is reachable from every platform; on
    /// non-Windows hosts the registered callback is never
    /// invoked (the OLE COM runtime is Windows-only). The
    /// data types themselves ([`OleDroppedData`],
    /// [`OleDropPosition`], [`crate::OleDropEffect`],
    /// [`OleDropError`]) are plain Rust data and are also
    /// reachable on every platform.
    ///
    /// # Errors
    ///
    /// Returns [`OleDropError::RegisterFailed`] if the COM
    /// runtime's `RegisterDragDrop` returns a non-zero
    /// `HRESULT`. The most common cause is that the window
    /// has already been registered with a different drop
    /// target — Win32 allows only one drop target per
    /// `HWND`. On failure, no drop target is stored in the
    /// frame (the callback closure is dropped, so the user
    /// must call again with a clean state to retry).
    ///
    /// # Replacement semantics
    ///
    /// A second call replaces the previous target: the old
    /// `OleDropTarget`'s `Drop` impl releases the IUnknown
    /// refcount (and `RevokeDragDrop` on Windows), so the
    /// COM runtime stops delivering `IDropTarget::*` calls
    /// to the old target before the new one is registered.
    /// The `Option<OleDropTarget>` slot always holds at
    /// most one target.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ru_wx::prelude::*;
    /// use ru_wx::{OleDroppedData, OleDropPosition};
    ///
    /// let frame = Frame::builder().with_title("OLE drop target").build();
    /// frame.set_ole_drop_callback(|data: OleDroppedData, pos: OleDropPosition| {
    ///     match data {
    ///         OleDroppedData::Files(paths) => {
    ///             for p in paths {
    ///                 println!("file: {} @ ({}, {})", p.display(), pos.x, pos.y);
    ///             }
    ///         }
    ///         OleDroppedData::Text(s) => println!("text: {:?}", s),
    ///         OleDroppedData::Other => println!("unknown format"),
    ///     }
    /// }).expect("first registration must succeed");
    /// ```
    pub fn set_ole_drop_callback<F>(&self, f: F) -> Result<(), OleDropError>
    where
        F: FnMut(OleDroppedData, OleDropPosition) + 'static,
    {
        // Initialise the OLE COM runtime exactly once per
        // process. The runtime treats repeat calls as a
        // no-op, so this is safe to do on every
        // `set_ole_drop_callback` call.
        #[cfg(target_os = "windows")]
        ole_dnd::ensure_ole_initialized();
        let mut target = OleDropTarget::new(Box::new(f));
        let hwnd = self.inner.borrow().hwnd;
        // `register` calls `RegisterDragDrop` on Windows
        // and is a no-op stub on non-Windows. On a `null`
        // HWND (e.g. the `for_testing` constructor), the
        // Win32 call returns an error; the `?` propagates
        // it and the new target is dropped (no entry is
        // stored in `FrameData`).
        target.register(hwnd)?;
        self.inner.borrow_mut().ole_drop_target = Some(target);
        Ok(())
    }

    /// Set window title
    pub fn set_title(&self, title: &str) {
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            let hwnd = self.inner.borrow().hwnd;
            let wide = to_wide(title);
            SetWindowTextW(hwnd, wide.as_ptr());
        }
    }

    /// Set window size. `w` and `h` are in **logical** (96-DPI)
    /// pixels — the value is converted to the frame's monitor's
    /// physical-pixel value internally, so a call to
    /// `set_size(800, 600)` always produces an 800×600-px
    /// window from the user's point of view, regardless of the
    /// current display scaling.
    pub fn set_size(&self, w: u32, h: u32) {
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            let hwnd = self.inner.borrow().hwnd;
            // Apply the same logical→physical conversion as
            // `build()`. Use the per-window DPI so the size stays
            // correct after the user drags the frame onto a
            // monitor with a different scaling.
            let dpi = get_dpi_for_window(hwnd);
            SetWindowPos(
                hwnd,
                std::ptr::null_mut(),
                0,
                0,
                dpi.scale(w as i32),
                dpi.scale(h as i32),
                SWP_NOMOVE | SWP_NOZORDER,
            );
            let _ = hwnd; // borrow dropped here
        }
    }

    /// Close the window
    pub fn close(&self) {
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            let hwnd = self.inner.borrow().hwnd;
            DestroyWindow(hwnd);
        }
    }

    /// Perform layout using the sizer.
    ///
    /// **Important:** the sizer is temporarily *taken out* of `FrameData` while
    /// we call `sizer.layout()`, so that the `RefCell` borrow is released before
    /// `MoveWindow` is invoked on child widgets. Without this, the synchronous
    /// `WM_ERASEBKGND` / `WM_SIZE` messages that Win32 emits during
    /// `MoveWindow` would re-enter the frame's `RefCell` and trigger a
    /// "RefCell already mutably borrowed" panic.
    fn do_layout(&self) {
        #[cfg(target_os = "windows")]
        {
            // Get the client area dimensions with a brief immutable borrow.
            let (w, h) = {
                let inner = self.inner.borrow();
                // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
                let mut rect: RECT = unsafe { std::mem::zeroed() };
                // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
                unsafe { GetClientRect(inner.hwnd, &mut rect) };
                (
                    (rect.right - rect.left) as u32,
                    (rect.bottom - rect.top) as u32,
                )
            };

            // Take the sizer out so the frame's RefCell borrow is released
            // BEFORE MoveWindow is called on any child widget.
            let mut sizer = self.inner.borrow_mut().sizer.take();
            if let Some(ref mut sizer) = sizer {
                sizer.layout(0, 0, w, h);
            }
            // Put the sizer back.
            self.inner.borrow_mut().sizer = sizer;
        }
    }

    /// Show the window and enter the message loop
    pub fn show(self) {
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            let hwnd = self.inner.borrow().hwnd;
            ShowWindow(hwnd, SW_SHOW);
            UpdateWindow(hwnd);

            // Trigger initial layout
            self.do_layout();

            // Build the Win32 HACCEL table from the registered accelerators.
            // The HACCEL is consumed by TranslateAcceleratorW inside the
            // message loop; we DestroyAcceleratorTable on exit.
            let h_accel: HACCEL = build_accelerator_table(&self.inner.borrow().accelerators);

            // Message loop
            let mut msg: MSG = std::mem::zeroed();
            while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {
                // TranslateAcceleratorW returns non-zero if it handled the
                // message (in which case it has already been translated
                // into a WM_COMMAND for the target window). If it returns
                // 0 we fall through to the standard translation/dispatch
                // path.
                if !h_accel.is_null() && TranslateAcceleratorW(hwnd, h_accel, &msg) != 0 {
                    continue;
                }
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }

            if !h_accel.is_null() {
                DestroyAcceleratorTable(h_accel);
            }
        }
    }
}

/// Build a Win32 `HACCEL` table from a list of `(Accelerator, command_id)`
/// pairs. Returns a null `HACCEL` if the list is empty (Win32 refuses to
/// pass `null` to `TranslateAcceleratorW` for some message types, but
/// `is_null()` is the only way to express "no table" in the FFI).
#[cfg(target_os = "windows")]
#[allow(clippy::not_unsafe_ptr_arg_deref)] // thin FFI wrapper, all pointers are owned ACCEL structs
fn build_accelerator_table(accels: &[(Accelerator, u16)]) -> HACCEL {
    if accels.is_empty() {
        return std::ptr::null_mut();
    }
    // Stack-allocate a fixed-size buffer if the count is small enough, or
    // a heap allocation otherwise. Win32's `CreateAcceleratorTableW` copies
    // the array, so the pointer only needs to remain valid for the call.
    let mut storage: Vec<ACCEL> = Vec::with_capacity(accels.len());
    for (a, cmd) in accels {
        storage.push(a.to_accel(*cmd));
    }
    // SAFETY: `storage` is a contiguous Vec of valid `ACCEL` values; the
    // count matches the Vec length. Win32 copies the table internally.
    unsafe { CreateAcceleratorTableW(storage.as_ptr(), storage.len() as i32) }
}

#[cfg(target_os = "windows")]
impl Window for Frame {
    fn hwnd(&self) -> HWND {
        self.hwnd()
    }
}

impl FrameBuilder {
    pub fn with_title(mut self, title: &str) -> Self {
        self.title = title.to_string();
        self
    }

    pub fn with_size(mut self, w: u32, h: u32) -> Self {
        self.width = w;
        self.height = h;
        self
    }

    pub fn with_position(mut self, x: i32, y: i32) -> Self {
        self.x = x;
        self.y = y;
        self
    }

    #[cfg(target_os = "windows")]
    pub fn build(self) -> Frame {
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            let hinstance = GetModuleHandleW(std::ptr::null());
            let class_name = to_wide("RuWxFrameClass");

            // Register window class (idempotent - will fail silently if already registered)
            let wc = WNDCLASSEXW {
                cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
                style: CS_HREDRAW | CS_VREDRAW,
                lpfnWndProc: Some(frame_wnd_proc),
                cbClsExtra: 0,
                cbWndExtra: 0,
                hInstance: hinstance,
                hIcon: LoadIconW(std::ptr::null_mut(), IDI_APPLICATION),
                hCursor: LoadCursorW(std::ptr::null_mut(), IDC_ARROW),
                hbrBackground: (COLOR_WINDOW + 1) as usize as HBRUSH,
                lpszMenuName: std::ptr::null(),
                lpszClassName: class_name.as_ptr(),
                hIconSm: std::ptr::null_mut(),
            };
            RegisterClassExW(&wc);

            // Create frame data
            let frame_data = Box::new(FrameData {
                hwnd: std::ptr::null_mut(),
                widgets: Vec::new(),
                command_handlers: HashMap::new(),
                notify_handlers: HashMap::new(),
                disp_info_handlers: HashMap::new(),
                dtn_handlers: HashMap::new(),
                tray_message_handlers: HashMap::new(),
                scroll_handlers: HashMap::new(),
                paint_handlers: Vec::new(),
                accelerators: Vec::new(),
                menu_bar: None,
                sizer: None,
                background_colour: Colour::LIGHT_GREY,
                on_resize: Vec::new(),
                on_close: None,
                drop_files_handler: None,
                ole_drop_target: None,
            });
            let frame_data_ptr = Box::into_raw(frame_data);

            let title_wide = to_wide(&self.title);
            // Apply DPI scaling: `with_size(w, h)` takes logical
            // (96-DPI) pixel values, but `CreateWindowExW` (and
            // every other Win32 coordinate-taking API) works in
            // physical pixels of the monitor the window lands on.
            // With `PerMonitorV2` awareness (declared by
            // `app.manifest`), the OS does NOT auto-scale window
            // coordinates — it is the application's job to pass
            // the physical-pixel value. Scaling here makes the
            // window the size the user asked for in 96-DPI units,
            // regardless of the monitor's actual DPI.
            //
            // We don't have an HWND yet, so we cannot ask
            // `GetDpiForWindow` what the actual monitor's DPI is.
            // The next best thing is `GetDpiForPoint` for the
            // requested (x, y). When the user did not pin a position
            // (`CW_USEDEFAULT`, the default) we fall back to
            // `get_dpi_for_point(0, 0)` which queries the primary
            // monitor. After the HWND is created we double-check
            // the actual DPI via `get_dpi_for_window` and resize
            // with `SetWindowPos` if it differs from our guess
            // (e.g. when the OS placed the window on a non-primary
            // monitor).
            let requested_x = if self.x == CW_USEDEFAULT { 0 } else { self.x };
            let requested_y = if self.y == CW_USEDEFAULT { 0 } else { self.y };
            let dpi = get_dpi_for_point(requested_x, requested_y);
            let physical_w = dpi.scale(self.width as i32);
            let physical_h = dpi.scale(self.height as i32);
            let hwnd = CreateWindowExW(
                0,
                class_name.as_ptr(),
                title_wide.as_ptr(),
                WS_OVERLAPPEDWINDOW,
                self.x,
                self.y,
                physical_w,
                physical_h,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                hinstance,
                std::ptr::null_mut(),
            );

            // Belt-and-suspenders: the OS may have placed the
            // window on a monitor whose DPI differs from our
            // initial guess (e.g. the user is on a multi-monitor
            // setup with mixed scalings, and the OS snapped the
            // window onto the high-DPI display even though we
            // guessed the primary). Re-read the actual per-window
            // DPI and, if it disagrees with our initial scale,
            // re-issue `SetWindowPos` so the size the user sees
            // is the size they asked for in 96-DPI units.
            let actual_dpi = get_dpi_for_window(hwnd);
            if actual_dpi.value() != dpi.value() {
                SetWindowPos(
                    hwnd,
                    std::ptr::null_mut(),
                    0,
                    0,
                    actual_dpi.scale(self.width as i32),
                    actual_dpi.scale(self.height as i32),
                    SWP_NOMOVE | SWP_NOZORDER,
                );
            }

            // Update the frame data with the hwnd
            (*frame_data_ptr).hwnd = hwnd;

            // Wrap in Rc<RefCell>
            let frame_data = Box::from_raw(frame_data_ptr);
            let inner = Rc::new(RefCell::new(*frame_data));

            // Store the Rc pointer in the window's user data for WndProc access
            let inner_clone = inner.clone();
            let raw = Rc::into_raw(inner_clone);
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, raw as isize);

            // Register the window as a Shell-level drop target. We do
            // this unconditionally (regardless of whether a drop
            // callback has been registered yet — `set_drop_files_callback`
            // is typically called *after* `build()` returns, just
            // before `show()` starts the message loop). The wndproc's
            // `WM_DROPFILES` arm checks whether a callback is
            // registered before invoking it, so an unconfigured
            // frame will silently no-op the drop message.
            //
            // We are already inside the outer `unsafe` block that
            // wraps the whole `build` body, so this call needs no
            // additional `unsafe` wrapper.
            DragAcceptFiles(hwnd, 1);

            Frame { inner }
        }
    }
}

/// Win32 Window Procedure
#[cfg(target_os = "windows")]
unsafe extern "system" fn frame_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_NOTIFY => {
            // WM_NOTIFY lparam points at an NMHDR; we use the idFrom field
            // to dispatch the notification to the registered handler for
            // that control id (used by the `Tab` control and by the
            // virtual-mode `ListCtrl` callback — see below).
            let nmhdr_ptr = lparam as *const NMHDR;
            if !nmhdr_ptr.is_null() {
                // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
                let id = unsafe { (*nmhdr_ptr).idFrom } as u16;
                let code = unsafe { (*nmhdr_ptr).code };
                let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA);
                if ptr != 0 {
                    // The build code stored the frame's
                    // `Rc<RefCell<FrameData>>` in `GWLP_USERDATA` via
                    // `Rc::into_raw(self.inner.clone())`, which leaves
                    // the strong count at 2 (the outer `Frame` still
                    // owns 1, and the "leaked" reference owns 1).
                    //
                    // `Rc::from_raw` does NOT increment the strong
                    // count on its own — it just reconstructs an `Rc`
                    // claiming the leaked slot. We need to bump the
                    // count first, otherwise the matching `drop(rc)`
                    // at the end of the arm would put the count at 1
                    // on the first dispatch, at 0 (and deallocate the
                    // backing storage) on the second, and every
                    // subsequent dispatch would be a use-after-free.
                    unsafe {
                        Rc::increment_strong_count(ptr as *const RefCell<FrameData>);
                    }
                    let rc = unsafe { Rc::from_raw(ptr as *const RefCell<FrameData>) };

                    // Virtual-list (`LVS_OWNERDATA`) dispatches carry a
                    // full `NMLVDISPINFOW` in `lparam`; the registered
                    // handler wants the whole pointer so it can
                    // interpret the request and write the response. All
                    // other `WM_NOTIFY` notifications go through the
                    // code-only `notify_handlers` path (used by the
                    // `Tab` control's `TCN_SELCHANGE`, the
                    // `ListCtrl`'s `LVN_ITEMCHANGED`, etc.).
                    // Three notification paths share the same
                    // `WM_NOTIFY` arm and the same `idFrom` keying
                    // space, so the dispatch has to choose between
                    // them based on the NMHDR `code`:
                    //
                    // * `LVN_GETDISPINFOW` (virtual-list disp-info)
                    //   carries a `NMLVDISPINFOW` in `lparam`; the
                    //   registered handler wants the whole pointer
                    //   so it can read the request fields and write
                    //   the response string. Routed to
                    //   `disp_info_handlers`.
                    // * `DTN_DATETIMECHANGE` (date-picker value
                    //   change) carries a `NMDATETIMECHANGE` in
                    //   `lparam`; the registered handler reads the
                    //   new `SYSTEMTIME` and the `GDT_VALID` /
                    //   `GDT_NONE` flag from the body. Routed to
                    //   `dtn_handlers`.
                    // * Everything else (the Tab control's
                    //   `TCN_SELCHANGE`, the ListCtrl's
                    //   `LVN_ITEMCHANGED`, etc.) goes through the
                    //   code-only `notify_handlers` path.
                    if code == crate::list_ctrl::LVN_GETDISPINFOW {
                        let handler = rc.borrow_mut().disp_info_handlers.remove(&id);
                        if let Some(mut h) = handler {
                            h(lparam);
                            rc.borrow_mut().disp_info_handlers.insert(id, h);
                        }
                    } else if code == crate::date_picker_ctrl::DTN_DATETIMECHANGE {
                        let handler = rc.borrow_mut().dtn_handlers.remove(&id);
                        if let Some(mut h) = handler {
                            h(lparam);
                            rc.borrow_mut().dtn_handlers.insert(id, h);
                        }
                    } else {
                        // Take the handler out so we don't hold a
                        // borrow across the user's callback.
                        let handler = rc.borrow_mut().notify_handlers.remove(&id);
                        if let Some(mut h) = handler {
                            h(code);
                            rc.borrow_mut().notify_handlers.insert(id, h);
                        }
                    }

                    drop(rc);
                }
            }
            0
        }
        WM_COMMAND => {
            let id = (wparam & 0xFFFF) as u16;
            let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA);
            if ptr != 0 {
                // The build code stored the frame's
                // `Rc<RefCell<FrameData>>` in `GWLP_USERDATA` via
                // `Rc::into_raw(self.inner.clone())`, which leaves
                // the strong count at 2. `Rc::from_raw` does NOT
                // increment the count by itself, so we have to bump
                // it manually before the reconstruction; otherwise
                // the matching `drop(rc)` at the end of this arm
                // would drop the count to 0 on the second
                // WM_COMMAND, freeing the backing storage and
                // turning the next dispatch into a use-after-free.
                unsafe {
                    Rc::increment_strong_count(ptr as *const RefCell<FrameData>);
                }
                let rc = unsafe { Rc::from_raw(ptr as *const RefCell<FrameData>) };

                // Take the handler out temporarily to avoid holding borrow during call
                let handler = rc.borrow_mut().command_handlers.remove(&id);

                // Call handler WITHOUT holding any borrow
                if let Some(mut h) = handler {
                    h();
                    // Put it back
                    rc.borrow_mut().command_handlers.insert(id, h);
                }

                drop(rc); // Decrements count back to original
            }
            0
        }
        WM_SIZE => {
            let width = (lparam & 0xFFFF) as u32;
            let height = ((lparam >> 16) & 0xFFFF) as u32;
            let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA);
            if ptr != 0 {
                // The build code stored the frame's
                // `Rc<RefCell<FrameData>>` in `GWLP_USERDATA` via
                // `Rc::into_raw(self.inner.clone())`, which leaves
                // the strong count at 2. `Rc::from_raw` does NOT
                // increment the count by itself, so we have to bump
                // it manually before the reconstruction; otherwise
                // the matching `drop(rc)` at the end of this arm
                // would drop the count to 0 on the second
                // WM_SIZE, freeing the backing storage and turning
                // the next dispatch into a use-after-free.
                unsafe {
                    Rc::increment_strong_count(ptr as *const RefCell<FrameData>);
                }
                let rc = unsafe { Rc::from_raw(ptr as *const RefCell<FrameData>) };

                // Take the sizer out, do the layout, put it back. This releases
                // the frame's RefCell borrow BEFORE MoveWindow is called on
                // child widgets, avoiding the re-entrancy panic described in
                // `do_layout`.
                let mut sizer = rc.borrow_mut().sizer.take();
                if let Some(ref mut sizer) = sizer {
                    sizer.layout(0, 0, width, height);
                }
                rc.borrow_mut().sizer = sizer;

                // Call all on_resize callbacks WITHOUT holding the borrow.
                // The callbacks are removed from the vec, invoked, then put back.
                // This is necessary because the callbacks may want to borrow the
                // frame's `RefCell` (e.g. `StatusBar`'s handler queries the
                // parent rect via `GetClientRect(GetParent(...))` and then
                // re-applies field widths through `SB_SETPARTS`).
                let mut data = rc.borrow_mut();
                let mut callbacks = std::mem::take(&mut data.on_resize);
                drop(data);
                for cb in callbacks.iter_mut() {
                    cb(width, height);
                }
                rc.borrow_mut().on_resize = callbacks;

                drop(rc);
            }
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }
        WM_ERASEBKGND => {
            let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA);
            if ptr != 0 {
                // Bump the strong count before `Rc::from_raw`;
                // see the WM_NOTIFY / WM_COMMAND / WM_SIZE arms
                // for the full rationale (without this, the
                // second dispatch would drop the count to 0 and
                // free the backing storage).
                unsafe {
                    Rc::increment_strong_count(ptr as *const RefCell<FrameData>);
                }
                let rc = unsafe { Rc::from_raw(ptr as *const RefCell<FrameData>) };
                let colour = rc.borrow().background_colour;
                drop(rc); // Release Rc before any Win32 painting calls

                let hdc = wparam as HDC;
                let mut rect: RECT = std::mem::zeroed();
                GetClientRect(hwnd, &mut rect);
                let brush = CreateSolidBrush(colour.to_colorref());
                FillRect(hdc, &rect, brush);
                DeleteObject(brush as _);
                return 1;
            }
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }
        WM_HSCROLL | WM_VSCROLL => {
            // Win32 routes `WM_HSCROLL` / `WM_VSCROLL` from SB_CTL
            // scroll bars to the parent frame, not to the control
            // itself. `lparam` is the scroll bar's `HWND` and
            // `wparam`'s low word is the SB_* request code
            // (`SB_LINEUP`, `SB_LINEDOWN`, `SB_PAGEUP`,
            // `SB_PAGEDOWN`, `SB_THUMBPOSITION`, `SB_THUMBTRACK`,
            // `SB_TOP`, `SB_BOTTOM`, `SB_ENDSCROLL`); the high word
            // carries the thumb position for `SB_THUMBPOSITION` /
            // `SB_THUMBTRACK` and 0 for the other codes.
            //
            // The dispatch follows the same pattern as
            // `WM_COMMAND` / `WM_NOTIFY` arms above: take the
            // handler out of the `RefCell` so the borrow is
            // released before the user's callback runs (the
            // callback may re-enter the frame, e.g. to update a
            // status bar; holding the borrow would panic on the
            // re-entry).
            let scroll_hwnd = lparam as HWND;
            let code = (wparam & 0xFFFF) as u16;
            let pos = ((wparam >> 16) & 0xFFFF) as i32;
            let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA);
            if ptr != 0 {
                // Bump the strong count before `Rc::from_raw`;
                // see the WM_NOTIFY / WM_COMMAND / WM_SIZE arms
                // for the full rationale.
                unsafe {
                    Rc::increment_strong_count(ptr as *const RefCell<FrameData>);
                }
                let rc = unsafe { Rc::from_raw(ptr as *const RefCell<FrameData>) };
                let handler = rc.borrow_mut().scroll_handlers.remove(&scroll_hwnd);
                if let Some(mut h) = handler {
                    h(code, pos);
                    rc.borrow_mut().scroll_handlers.insert(scroll_hwnd, h);
                }
                drop(rc);
            }
            0
        }
        WM_PAINT => {
            // `BeginPaint` / `EndPaint` pair around the user's
            // callback so the user can issue any GDI drawing call
            // on the DC without managing the `PAINTSTRUCT`
            // themselves. The callback receives the `HDC` from
            // `BeginPaint` cast to `isize` (the `DC` wrappers —
            // `PaintDC`, `WindowDC`, `MemoryDC` — take a `HDC` and
            // re-interpret it the same way). If no paint handler
            // is registered, the default window proc draws the
            // background and any sizer-driven child widgets.
            let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA);
            if ptr != 0 {
                // Bump the strong count before `Rc::from_raw`;
                // see the WM_NOTIFY / WM_COMMAND / WM_SIZE arms
                // for the full rationale. WM_PAINT does the
                // reconstruction twice (once before `BeginPaint`,
                // once after `EndPaint`) to release the
                // `RefCell` borrow across the painting
                // callbacks; each reconstruction has to be
                // matched with its own `increment_strong_count`.
                unsafe {
                    Rc::increment_strong_count(ptr as *const RefCell<FrameData>);
                }
                let rc = unsafe { Rc::from_raw(ptr as *const RefCell<FrameData>) };

                // Take the handler list out so the borrow is
                // released before any user callback runs.
                let mut handlers = std::mem::take(&mut rc.borrow_mut().paint_handlers);
                drop(rc); // re-entry safety: drop before BeginPaint

                let mut ps: PAINTSTRUCT = std::mem::zeroed();
                // SAFETY: `hwnd` is the frame's HWND (passed by Win32
                // in the WM_PAINT dispatch); `ps` is a valid
                // stack-allocated `PAINTSTRUCT` buffer.
                let hdc = unsafe { BeginPaint(hwnd, &mut ps) };
                for cb in handlers.iter_mut() {
                    cb(hdc as isize);
                }
                // SAFETY: paired with the matching `BeginPaint`
                // call above; `ps` was filled in by Win32.
                unsafe { EndPaint(hwnd, &ps) };

                // Put the handler list back. The pattern is
                // the same as the first reconstruction: bump
                // the count, then `from_raw` (NOT just `from_raw`
                // alone — see the WM_NOTIFY arm for why).
                let raw = GetWindowLongPtrW(hwnd, GWLP_USERDATA);
                debug_assert!(raw != 0);
                unsafe {
                    Rc::increment_strong_count(raw as *const RefCell<FrameData>);
                }
                let rc = unsafe { Rc::from_raw(raw as *const RefCell<FrameData>) };
                rc.borrow_mut().paint_handlers = handlers;
                drop(rc);
                return 0;
            }
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }
        WM_DESTROY => {
            // Clean up the Rc
            let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA);
            if ptr != 0 {
                let _ = Rc::from_raw(ptr as *const RefCell<FrameData>);
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
            }
            PostQuitMessage(0);
            0
        }
        WM_DROPFILES => {
            // `wparam` carries the `HDROP` Shell32 hands us when the
            // user drops one or more files from Explorer onto the
            // frame. The handle stays valid until we call
            // `DragFinish` on it (see `drop_target::finish_drop`).
            //
            // The pattern below matches every other callback dispatch
            // in this wndproc:
            //   1. Reach the `FrameData` via `GWLP_USERDATA`.
            //   2. Bump the strong count with
            //      `Rc::increment_strong_count` and then
            //      `Rc::from_raw` to reconstruct the strong `Rc` we
            //      stored at build-time. `from_raw` does NOT
            //      increment the count on its own, so the bump is
            //      what keeps the count above 0 across multiple
            //      dispatches (without it, the second WM_DROPFILES
            //      would drop the count to 0 and free the backing
            //      storage). The matching `drop(rc)` at the end of
            //      the arm brings the count back to its
            //      pre-dispatch value.
            //   3. `.take()` the handler out of the `RefCell` so the
            //      borrow is released before the user's callback runs
            //      (the callback may re-enter the frame, e.g. to
            //      update a status bar; holding the borrow would
            //      panic on the re-entry).
            //   4. Invoke the handler.
            //   5. Put the handler back.
            //
            // `DragFinish` is called *unconditionally* at the end so
            // the Shell's internal storage is always released, even
            // if no callback was registered or the userdata pointer
            // is null (the latter can happen during very early
            // teardown — we still need to free the Shell handle).
            let hdrop: HDROP = wparam as HDROP;
            let paths = drop_target::extract_paths_from_hdrop(hdrop);
            let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA);
            if ptr != 0 {
                unsafe {
                    Rc::increment_strong_count(ptr as *const RefCell<FrameData>);
                }
                let rc = unsafe { Rc::from_raw(ptr as *const RefCell<FrameData>) };
                if !paths.is_empty() {
                    let handler = rc.borrow_mut().drop_files_handler.take();
                    if let Some(mut h) = handler {
                        h(DroppedFiles::new(paths));
                        rc.borrow_mut().drop_files_handler = Some(h);
                    }
                }
                drop(rc);
            }
            drop_target::finish_drop(hdrop);
            0
        }
        msg if msg >= WM_APP => {
            // User-defined message — used by `IconTray` for shell
            // notification area callbacks. Dispatch to the registered
            // handler (if any) with the message's lparam (which carries
            // the mouse event / NIN_* notification code).
            let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA);
            if ptr != 0 {
                // Bump the strong count before `Rc::from_raw`;
                // see the WM_NOTIFY / WM_COMMAND / WM_SIZE arms
                // for the full rationale.
                unsafe {
                    Rc::increment_strong_count(ptr as *const RefCell<FrameData>);
                }
                let rc = unsafe { Rc::from_raw(ptr as *const RefCell<FrameData>) };
                let handler = rc.borrow_mut().tray_message_handlers.remove(&msg);
                if let Some(mut h) = handler {
                    h(lparam as u32);
                    rc.borrow_mut().tray_message_handlers.insert(msg, h);
                }
                drop(rc);
                return 0;
            }
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

#[cfg(test)]
mod tests {
    //! Unit tests for the platform-agnostic part of the [`Frame`]
    //! public surface. They use the [`Frame::for_testing`]
    //! constructor (which produces a `Frame` with a `null` `HWND`)
    //! so they can run on any host without a real Win32 message
    //! pump.
    //!
    //! The tests cover:
    //!
    //! * accelerator registration / iteration / duplicate handling
    //! * command-handler and notify-handler registration
    //! * sizer storage (no `MoveWindow` is issued because the
    //!   `null` `HWND` short-circuits `do_layout` before any
    //!   platform call)
    //! * the null-`HWND` fallback path of [`Frame::dpi`] /
    //!   [`Frame::scale_factor`]
    //!
    //! Anything that would normally route through the Win32
    //! `WndProc` (real `WM_COMMAND` dispatch, real `WM_SIZE`
    //! layout) is intentionally out of scope here; the
    //! `examples/showcase_all.rs` binary is the integration test
    //! for those paths.

    use super::*;
    use crate::accelerator::Accelerator;
    use crate::menu::{Menu, MenuBar};
    use crate::sizer::{BoxSizer, Orientation};
    use std::rc::Rc;

    #[test]
    fn for_testing_starts_with_empty_state() {
        let f = Frame::for_testing();
        assert!(f.accelerators().is_empty());
        assert!(f.inner.borrow().command_handlers.is_empty());
        assert!(f.inner.borrow().notify_handlers.is_empty());
        assert!(f.inner.borrow().tray_message_handlers.is_empty());
        assert!(f.inner.borrow().sizer.is_none());
        assert_eq!(f.inner.borrow().background_colour, Colour::LIGHT_GREY);
        assert!(
            f.inner.borrow().drop_files_handler.is_none(),
            "freshly-built frame must have no drop-files handler"
        );
    }

    // ---------- Accelerator registration ----------

    #[test]
    fn register_accelerator_preserves_order() {
        let f = Frame::for_testing();
        let a = Accelerator::parse("Ctrl+S").unwrap();
        let b = Accelerator::parse("Ctrl+O").unwrap();
        let c = Accelerator::parse("F5").unwrap();

        f.register_accelerator(a, 1);
        f.register_accelerator(b, 2);
        f.register_accelerator(c, 3);

        let list = f.accelerators();
        assert_eq!(list.len(), 3);
        assert_eq!(list[0], (a, 1));
        assert_eq!(list[1], (b, 2));
        assert_eq!(list[2], (c, 3));
    }

    #[test]
    fn register_accelerator_accepts_duplicates() {
        // The doc-comment on `register_accelerator` explicitly says
        // duplicate bindings are accepted (matching the Win32
        // HACCEL lookup order: first match wins). Lock that
        // behaviour in here so a future refactor cannot silently
        // change it.
        let f = Frame::for_testing();
        let a = Accelerator::parse("Ctrl+S").unwrap();
        f.register_accelerator(a, 100);
        f.register_accelerator(a, 200);
        f.register_accelerator(a, 300);

        let list = f.accelerators();
        assert_eq!(list.len(), 3);
        assert_eq!(list[0].1, 100);
        assert_eq!(list[1].1, 200);
        assert_eq!(list[2].1, 300);
    }

    #[test]
    fn accelerators_clone_is_isolated() {
        // The Vec returned by `accelerators()` is a fresh clone, so
        // mutating the frame afterwards must not retroactively
        // change the returned Vec. This is the property user code
        // relies on for "snapshot the bindings, then re-register a
        // few" workflows.
        let f = Frame::for_testing();
        let a = Accelerator::parse("Ctrl+S").unwrap();
        f.register_accelerator(a, 1);

        let snapshot = f.accelerators();
        assert_eq!(snapshot.len(), 1);

        f.register_accelerator(a, 2);
        assert_eq!(
            snapshot.len(),
            1,
            "snapshot must not observe the second registration"
        );
        assert_eq!(f.accelerators().len(), 2);
    }

    // ---------- Accelerator rebinding (v0.5.1) ----------

    #[test]
    fn unregister_accelerator_returns_false_when_absent() {
        // On a freshly-built frame there is nothing to remove.
        let f = Frame::for_testing();
        let a = Accelerator::parse("Ctrl+S").unwrap();
        assert!(!f.unregister_accelerator(a));
        assert!(f.accelerators().is_empty());
    }

    #[test]
    fn unregister_accelerator_returns_true_when_present() {
        let f = Frame::for_testing();
        let a = Accelerator::parse("Ctrl+S").unwrap();
        f.register_accelerator(a, 42);
        assert!(f.unregister_accelerator(a));
        assert!(f.accelerators().is_empty());
    }

    #[test]
    fn unregister_accelerator_preserves_relative_order() {
        // The remaining entries must keep their positions; we use
        // `Vec::retain` for that, but the property is what we care
        // about (the Win32 HACCEL lookup order is "first match
        // wins", so a reordering here would be a silent behaviour
        // change).
        let f = Frame::for_testing();
        let a = Accelerator::parse("Ctrl+S").unwrap();
        let b = Accelerator::parse("Ctrl+O").unwrap();
        let c = Accelerator::parse("F5").unwrap();
        f.register_accelerator(a, 1);
        f.register_accelerator(b, 2);
        f.register_accelerator(c, 3);

        assert!(f.unregister_accelerator(b));

        let list = f.accelerators();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0], (a, 1));
        assert_eq!(list[1], (c, 3));
    }

    #[test]
    fn unregister_accelerator_removes_only_first_duplicate() {
        // The doc-comment on `unregister_accelerator` says it
        // removes the first match and leaves the rest, matching the
        // duplicate-tolerance of `register_accelerator`. Lock that
        // down here.
        let f = Frame::for_testing();
        let a = Accelerator::parse("Ctrl+S").unwrap();
        f.register_accelerator(a, 100);
        f.register_accelerator(a, 200);
        f.register_accelerator(a, 300);

        assert!(f.unregister_accelerator(a));
        let list = f.accelerators();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].1, 200);
        assert_eq!(list[1].1, 300);
    }

    #[test]
    fn clear_accelerators_empties_the_list() {
        let f = Frame::for_testing();
        f.register_accelerator(Accelerator::parse("Ctrl+S").unwrap(), 1);
        f.register_accelerator(Accelerator::parse("F5").unwrap(), 2);
        f.register_accelerator(Accelerator::parse("Alt+F4").unwrap(), 3);
        assert_eq!(f.accelerators().len(), 3);

        f.clear_accelerators();
        assert!(f.accelerators().is_empty());
    }

    #[test]
    fn clear_accelerators_on_empty_is_a_noop() {
        let f = Frame::for_testing();
        f.clear_accelerators();
        assert!(f.accelerators().is_empty());
        f.clear_accelerators(); // idempotent
        assert!(f.accelerators().is_empty());
    }

    #[test]
    fn replace_accelerator_returns_false_when_old_absent() {
        let f = Frame::for_testing();
        let old = Accelerator::parse("Ctrl+S").unwrap();
        let new = Accelerator::parse("Ctrl+Shift+S").unwrap();
        assert!(!f.replace_accelerator(old, new, 42));
        assert!(
            f.accelerators().is_empty(),
            "absent-old replace must not add an entry"
        );
    }

    #[test]
    fn replace_accelerator_swaps_in_place() {
        // The replacement must take the same slot in the list, not
        // append to the end; otherwise the Win32 "first match wins"
        // ordering would change silently.
        let f = Frame::for_testing();
        let a = Accelerator::parse("Ctrl+S").unwrap();
        let b = Accelerator::parse("Ctrl+O").unwrap();
        let old = Accelerator::parse("F5").unwrap();
        let new = Accelerator::parse("Shift+F5").unwrap();
        f.register_accelerator(a, 1);
        f.register_accelerator(b, 2);
        f.register_accelerator(old, 3);
        f.register_accelerator(b, 22); // duplicate of b, on purpose

        assert!(f.replace_accelerator(old, new, 33));

        let list = f.accelerators();
        assert_eq!(list.len(), 4);
        assert_eq!(list[0], (a, 1));
        assert_eq!(list[1], (b, 2));
        assert_eq!(list[2], (new, 33));
        assert_eq!(list[3], (b, 22));
    }

    #[test]
    fn replace_accelerator_handles_duplicates_of_old() {
        // If `old` is registered more than once, the first match is
        // replaced; the remaining duplicates are left in place. This
        // matches `unregister_accelerator`'s "first match wins"
        // semantics.
        let f = Frame::for_testing();
        let old = Accelerator::parse("Ctrl+S").unwrap();
        let new = Accelerator::parse("Ctrl+Shift+S").unwrap();
        f.register_accelerator(old, 100);
        f.register_accelerator(old, 200);
        f.register_accelerator(old, 300);

        assert!(f.replace_accelerator(old, new, 999));
        let list = f.accelerators();
        assert_eq!(list.len(), 3);
        assert_eq!(list[0], (new, 999));
        assert_eq!(list[1], (old, 200));
        assert_eq!(list[2], (old, 300));
    }

    #[test]
    fn rebind_three_step_workflow() {
        // A realistic end-to-end use case: register, inspect,
        // rebind, inspect, clear, inspect. This is the kind of
        // sequence an "Options" dialog might drive at runtime.
        let f = Frame::for_testing();
        let save_old = Accelerator::parse("Ctrl+S").unwrap();
        let save_new = Accelerator::parse("Ctrl+Shift+S").unwrap();
        let open = Accelerator::parse("Ctrl+O").unwrap();
        let quit = Accelerator::parse("Alt+F4").unwrap();

        f.register_accelerator(save_old, 1);
        f.register_accelerator(open, 2);
        f.register_accelerator(quit, 3);
        assert_eq!(f.accelerators().len(), 3);

        // User re-binds "save" to Ctrl+Shift+S in the Options
        // dialog. The Ctrl+S entry is gone afterwards.
        assert!(f.replace_accelerator(save_old, save_new, 1));
        assert_eq!(f.accelerators().len(), 3);
        assert!(!f.accelerators().iter().any(|(a, _)| *a == save_old));
        assert!(f
            .accelerators()
            .iter()
            .any(|(a, id)| *a == save_new && *id == 1));

        // Then they decide to clear the accelerator table entirely
        // (e.g. switching to a menu-only UI). The frame's
        // command-handler map is unaffected by this; only the
        // accelerator table is.
        f.clear_accelerators();
        assert!(f.accelerators().is_empty());
        assert_eq!(f.inner.borrow().command_handlers.len(), 0);
    }

    // ---------- Handler registration ----------

    #[test]
    fn register_command_handler_appears_in_map() {
        let f = Frame::for_testing();
        let called = Rc::new(std::cell::Cell::new(false));
        let called_clone = called.clone();
        f.register_command_handler(42, Box::new(move || called_clone.set(true)));

        let mut h = f.inner.borrow_mut().command_handlers.remove(&42).unwrap();
        h();
        assert!(called.get());
    }

    #[test]
    fn register_command_handler_overwrites_previous() {
        // A second call for the same id must replace the first
        // handler (matching the HashMap::insert semantics that
        // `register_command_handler` uses internally).
        let f = Frame::for_testing();
        let first_calls = Rc::new(std::cell::Cell::new(0u32));
        let second_calls = Rc::new(std::cell::Cell::new(0u32));
        let fc1 = first_calls.clone();
        let sc1 = second_calls.clone();
        f.register_command_handler(7, Box::new(move || fc1.set(fc1.get() + 1)));
        f.register_command_handler(7, Box::new(move || sc1.set(sc1.get() + 1)));

        let mut h = f.inner.borrow_mut().command_handlers.remove(&7).unwrap();
        h();
        assert_eq!(first_calls.get(), 0);
        assert_eq!(second_calls.get(), 1);
    }

    #[test]
    fn register_notify_handler_appears_in_map() {
        let f = Frame::for_testing();
        let seen = Rc::new(std::cell::Cell::new(0u32));
        let seen_clone = seen.clone();
        f.register_notify_handler(9, Box::new(move |code| seen_clone.set(code)));

        let mut h = f.inner.borrow_mut().notify_handlers.remove(&9).unwrap();
        // TVN_SELCHANGED = 0xFFFF_FFFE_signed but here we just pass
        // a sentinel 0xABCD and confirm the closure receives it.
        h(0xABCD);
        assert_eq!(seen.get(), 0xABCD);
    }

    #[test]
    fn unregister_tray_message_handler_removes_entry() {
        let f = Frame::for_testing();
        f.register_tray_message_handler(0x8001, Box::new(|_| {}));
        assert!(f.inner.borrow().tray_message_handlers.contains_key(&0x8001));
        f.unregister_tray_message_handler(0x8001);
        assert!(!f.inner.borrow().tray_message_handlers.contains_key(&0x8001));
    }

    // ---------- Sizer storage ----------

    #[test]
    fn set_sizer_stores_and_can_be_replaced() {
        let f = Frame::for_testing();
        let mut s = BoxSizer::horizontal();
        s.set_padding(7);
        f.set_sizer(s);
        assert!(f.inner.borrow().sizer.is_some());
        assert_eq!(f.inner.borrow().sizer.as_ref().unwrap().padding(), 7);
        assert!(matches!(
            f.inner.borrow().sizer.as_ref().unwrap().orientation(),
            Orientation::Horizontal
        ));

        // Replacing the sizer is allowed (it just drops the old one).
        let s2 = BoxSizer::vertical();
        f.set_sizer(s2);
        let sizer_ref = f.inner.borrow();
        let orientation = sizer_ref.sizer.as_ref().unwrap().orientation();
        assert!(matches!(orientation, Orientation::Vertical));
    }

    // ---------- DPI / scale_factor (null HWND) ----------

    #[test]
    #[cfg(target_os = "windows")]
    fn dpi_falls_back_to_system_dpi_for_null_hwnd() {
        // The frame's HWND is null in the test constructor, so
        // `get_dpi_for_window(null)` must delegate to
        // `get_system_dpi()`. We don't assert a specific value
        // (the system DPI varies by display configuration), but
        // we do assert that the value is at least the standard
        // 96-DPI baseline and is non-zero.
        let f = Frame::for_testing();
        let d = f.dpi();
        assert!(
            d.value() >= 96,
            "system DPI must be >= 96, got {}",
            d.value()
        );
        assert!(d.scale_factor() >= 1.0);
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn scale_factor_matches_dpi_for_null_hwnd() {
        // The shortcut must round-trip through dpi() without a
        // divide-by-zero or a NaN slipping in.
        let f = Frame::for_testing();
        let d = f.dpi();
        let s = f.scale_factor();
        assert_eq!(s, d.scale_factor());
        assert!(s.is_finite());
        assert!(s > 0.0);
    }

    // ---------- Menu-bar integration (v0.5.4) ----------
    //
    // These tests cover the bug fix that ties the frame's
    // accelerator table to the menubar's visible labels. Before
    // v0.5.4 the three accelerator mutators (`register`,
    // `unregister`, `replace`, `clear`) only touched the
    // `accelerators: Vec<(Accelerator, u16)>` — the menu kept
    // showing the old shortcut text. The fix is that each mutator
    // now also calls `MenuBar::update_item_shortcut`, which both
    // mutates the in-memory `MenuItem.shortcut` field and refreshes
    // the Win32 label via `ModifyMenuW`.
    //
    // The tests verify the in-memory state by reaching into the
    // stored `MenuBar` via its `pub(crate) menus()` accessor. The
    // Win32 label is left to the manual smoke-test in
    // `examples/showcase_all.rs` (and the visual end-to-end in
    // `wxwin11_demo`).

    /// Build a frame + menubar with a single shortcut-bearing
    /// item and return the frame, the item id, and the original
    /// accelerator so the test can assert against it.
    fn frame_with_shortcut_item(label: &str, accel: &str) -> (Frame, u16, Accelerator) {
        let f = Frame::for_testing();
        let mut menu = Menu::new("File");
        let accel = Accelerator::parse(accel).unwrap();
        let id = menu.append_with_shortcut(label, accel, &f, || {});

        let mut bar = MenuBar::new();
        bar.append(menu);
        f.set_menu_bar(bar);
        (f, id, accel)
    }

    #[test]
    fn set_menu_bar_stores_the_menubar_in_frame_data() {
        // `set_menu_bar` must keep an owned copy of the menubar
        // inside `FrameData`; otherwise the accelerator mutators
        // would have nothing to update.
        let f = Frame::for_testing();
        assert!(f.inner.borrow().menu_bar.is_none());

        let mut menu = Menu::new("File");
        menu.append("Open", &f, || {});
        let mut bar = MenuBar::new();
        bar.append(menu);
        f.set_menu_bar(bar);

        assert!(f.inner.borrow().menu_bar.is_some());
        assert_eq!(f.inner.borrow().menu_bar.as_ref().unwrap().menus().len(), 1);
    }

    #[test]
    fn set_menu_bar_replaces_a_previous_menubar() {
        // A second `set_menu_bar` call must drop the previous
        // menubar (its `Drop` releases the underlying HMENUs) and
        // install the new one.
        let f = Frame::for_testing();
        let mut first = MenuBar::new();
        first.append(Menu::new("A"));
        f.set_menu_bar(first);

        let mut second = MenuBar::new();
        second.append(Menu::new("B"));
        f.set_menu_bar(second);

        let data = f.inner.borrow();
        let bar = data.menu_bar.as_ref().unwrap();
        assert_eq!(bar.menus().len(), 1);
        assert_eq!(bar.menus()[0].title(), "B");
    }

    #[test]
    fn replace_accelerator_refreshes_menu_label() {
        // The bug-fixed behaviour: after `replace_accelerator`,
        // the menu item's in-memory `shortcut` must reflect the
        // new accelerator, not the old one.
        let (f, id, old) = frame_with_shortcut_item("Save", "Ctrl+S");
        let new = Accelerator::parse("Ctrl+Shift+S").unwrap();

        assert!(f.replace_accelerator(old, new, id));

        let data = f.inner.borrow();
        let bar = data.menu_bar.as_ref().unwrap();
        let item = bar.menus()[0].item(id).expect("menu item must still exist");
        assert_eq!(
            item.shortcut,
            Some(new),
            "replace_accelerator must rewrite the menu's stored shortcut"
        );
    }

    #[test]
    fn unregister_accelerator_clears_menu_label() {
        // The bug-fixed behaviour: after `unregister_accelerator`,
        // the menu item's in-memory `shortcut` must be cleared.
        let (f, _id, accel) = frame_with_shortcut_item("Save", "Ctrl+S");
        let bar = f.inner.borrow();
        let id = bar.menu_bar.as_ref().unwrap().menus()[0].items()[0].id;
        drop(bar);

        assert!(f.unregister_accelerator(accel));

        let bar = f.inner.borrow();
        let item = bar.menu_bar.as_ref().unwrap().menus()[0].item(id).unwrap();
        assert!(
            item.shortcut.is_none(),
            "unregister_accelerator must clear the menu's stored shortcut"
        );
    }

    #[test]
    fn clear_accelerators_clears_all_menu_labels() {
        // Build a frame with two shortcut-bearing items, then
        // call `clear_accelerators`. Both menu items must end up
        // with `shortcut: None`.
        let f = Frame::for_testing();
        let mut menu = Menu::new("File");
        let id_save =
            menu.append_with_shortcut("Save", Accelerator::parse("Ctrl+S").unwrap(), &f, || {});
        let id_open =
            menu.append_with_shortcut("Open", Accelerator::parse("Ctrl+O").unwrap(), &f, || {});
        let mut bar = MenuBar::new();
        bar.append(menu);
        f.set_menu_bar(bar);

        f.clear_accelerators();

        let data = f.inner.borrow();
        let submenu = &data.menu_bar.as_ref().unwrap().menus()[0];
        assert!(submenu.item(id_save).unwrap().shortcut.is_none());
        assert!(submenu.item(id_open).unwrap().shortcut.is_none());
    }

    #[test]
    fn replace_accelerator_without_menubar_remains_safe() {
        // A frame with no attached menubar must still accept
        // `replace_accelerator` without panicking — the mutator
        // just sees the `menu_bar == None` branch and skips the
        // menu refresh.
        let f = Frame::for_testing();
        let old = Accelerator::parse("Ctrl+S").unwrap();
        let new = Accelerator::parse("Ctrl+Shift+S").unwrap();
        f.register_accelerator(old, 42);
        assert!(f.replace_accelerator(old, new, 42));
        assert!(f
            .accelerators()
            .iter()
            .any(|(a, id)| *a == new && *id == 42));
    }

    #[test]
    fn unregister_accelerator_without_menubar_remains_safe() {
        let f = Frame::for_testing();
        let a = Accelerator::parse("Ctrl+S").unwrap();
        f.register_accelerator(a, 42);
        assert!(f.unregister_accelerator(a));
        assert!(f.accelerators().is_empty());
    }

    #[test]
    fn clear_accelerators_without_menubar_remains_safe() {
        let f = Frame::for_testing();
        f.register_accelerator(Accelerator::parse("Ctrl+S").unwrap(), 1);
        f.clear_accelerators();
        assert!(f.accelerators().is_empty());
    }

    // ---------- Drop-files handler (v0.5.5) ----------

    #[test]
    fn for_testing_starts_without_drop_files_handler() {
        // Mirrors `for_testing_starts_with_empty_state` for the new
        // field: a `Frame::for_testing()` (and by extension a
        // freshly-built one) must have no drop-files handler
        // registered, so WM_DROPFILES is a no-op until the user
        // opts in via `set_drop_files_callback`.
        let f = Frame::for_testing();
        assert!(f.inner.borrow().drop_files_handler.is_none());
    }

    #[test]
    fn set_drop_files_callback_stores_handler() {
        // After registering, the slot must hold a `Some(_)`. We
        // don't have a real `WM_DROPFILES` here (no HWND), so the
        // most we can prove at this level is "the option flipped
        // from None to Some".
        let f = Frame::for_testing();
        f.set_drop_files_callback(|_files| {});
        assert!(f.inner.borrow().drop_files_handler.is_some());
    }

    #[test]
    fn set_drop_files_callback_replaces_previous() {
        // The docstring on `set_drop_files_callback` says the
        // previous handler is dropped (i.e. replaced, not
        // appended). Lock that down: the slot must still hold
        // exactly one handler after two registrations.
        let f = Frame::for_testing();
        f.set_drop_files_callback(|_files| {});
        f.set_drop_files_callback(|_files| {});
        assert!(f.inner.borrow().drop_files_handler.is_some());
    }

    #[test]
    fn set_drop_files_callback_keeps_handler_alive_across_borrows() {
        // The handler is a `Box<dyn FnMut(...)>`. If it held a
        // reference into the frame's own `RefCell` it would
        // conflict with the dispatch borrow in `WM_DROPFILES`.
        // The current design stores the handler as owned state,
        // so re-borrowing the frame afterwards must not panic
        // (RefCell aliasing rule).
        let f = Frame::for_testing();
        f.set_drop_files_callback(|_files| {});
        // Read-only borrows of unrelated fields must work.
        let _accels = f.accelerators();
        let _h = f.inner.borrow().drop_files_handler.is_some();
    }

    #[test]
    fn set_drop_files_callback_accepts_capturing_closure() {
        // The bound is `FnMut + 'static`, so a closure that
        // captures local state must be accepted. We use a
        // `Cell<bool>` because the closure is `FnMut` (it might
        // be called more than once if Shell sends multiple
        // `WM_DROPFILES` events). The real call is impossible to
        // exercise from a unit test (no HWND), but we can at
        // least prove the registration path accepts the capture.
        use std::cell::Cell;
        let f = Frame::for_testing();
        let called = Cell::new(false);
        f.set_drop_files_callback(move |_files| {
            called.set(true);
        });
        assert!(f.inner.borrow().drop_files_handler.is_some());
    }

    // ---------- Disp-info handler (v0.5.6) ----------
    //
    // The `register_disp_info_handler` method backs the
    // `LVS_OWNERDATA` virtual-mode `ListCtrl` callback. The
    // handler receives the full `lparam` (a `*mut
    // NMLVDISPINFOW`) so the wrapper can both read the request
    // and write the response. These tests pin the registration
    // contract: the entry is stored, re-registration replaces,
    // the closure can capture state, the signature matches,
    // and the disp-info map is independent from the
    // code-only `notify_handlers` map. The actual `WM_NOTIFY`
    // dispatch split between the two maps is in
    // `frame_wnd_proc`'s `WM_NOTIFY` arm.

    /// Registering a disp-info handler must insert an entry
    /// into the disp-info handler map keyed by the supplied
    /// control id.
    #[test]
    fn register_disp_info_handler_stores_entry() {
        let f = Frame::for_testing();
        f.register_disp_info_handler(0x4001, Box::new(|_lparam| {}));
        assert!(f.inner.borrow().disp_info_handlers.contains_key(&0x4001));
    }

    /// Re-registering a disp-info handler for the same id
    /// must replace the previous one (dropping the old
    /// `Box<dyn FnMut>`). This matches the `on_get_disp_info`
    /// "one owner" model documented on the method.
    #[test]
    fn register_disp_info_handler_replaces_previous() {
        let f = Frame::for_testing();
        f.register_disp_info_handler(0x4002, Box::new(|_| {}));
        f.register_disp_info_handler(0x4002, Box::new(|_| {}));
        // Only one entry remains for that id
        // (HashMap::insert is upsert-by-key).
        assert_eq!(f.inner.borrow().disp_info_handlers.len(), 1);
        assert!(f.inner.borrow().disp_info_handlers.contains_key(&0x4002));
    }

    /// Pin the public method's signature. A future change
    /// (e.g. a different return type or a borrowed parameter)
    /// would fail to compile here.
    #[test]
    #[allow(clippy::type_complexity)]
    fn signature_register_disp_info_handler() {
        let _: fn(&Frame, u16, Box<dyn FnMut(isize)>) = Frame::register_disp_info_handler;
    }

    /// The handler closure must be able to capture local
    /// state. This is a real use case: the caller typically
    /// wants to index into a model and the model is owned
    /// outside the frame.
    #[test]
    fn disp_info_handler_accepts_capturing_closure() {
        use std::cell::Cell;
        use std::rc::Rc;
        let f = Frame::for_testing();
        let count = Rc::new(Cell::new(0_u32));
        // Clone the `Rc` (not the `Cell`) so the outer scope
        // can still observe the count after the closure has
        // taken ownership of one of the references.
        let count_for_closure = count.clone();
        f.register_disp_info_handler(
            0x4003,
            Box::new(move |_lparam| {
                count_for_closure.set(count_for_closure.get() + 1);
            }),
        );
        // Direct invocation (no real Win32 message pump) —
        // just prove the closure was stored and can be
        // re-borrowed.
        let mut h = f
            .inner
            .borrow_mut()
            .disp_info_handlers
            .remove(&0x4003)
            .unwrap();
        h(0);
        h(0);
        f.inner.borrow_mut().disp_info_handlers.insert(0x4003, h);
        assert_eq!(count.get(), 2);
    }

    /// The disp-info map and the notify map are independent:
    /// registering a handler in one must not affect the other.
    /// The `WM_NOTIFY` arm in `frame_wnd_proc` uses the two
    /// maps for two different notification codes
    /// (`LVN_GETDISPINFOW` vs. everything else) and the
    /// split must stay disjoint.
    #[test]
    fn disp_info_and_notify_maps_are_independent() {
        let f = Frame::for_testing();
        f.register_disp_info_handler(0x4004, Box::new(|_| {}));
        f.register_notify_handler(0x4004, Box::new(|_| {}));
        assert!(f.inner.borrow().disp_info_handlers.contains_key(&0x4004));
        assert!(f.inner.borrow().notify_handlers.contains_key(&0x4004));
        assert_eq!(f.inner.borrow().disp_info_handlers.len(), 1);
        assert_eq!(f.inner.borrow().notify_handlers.len(), 1);
    }

    // ── DTN handler (DatePickerCtrl) ────────────────────────────────
    //
    // The `dtn_handlers` map was added in v0.5.7 alongside
    // `DatePickerCtrl::on_date_change` to back the
    // `DTN_DATETIMECHANGE` notification that the
    // `SysDateTimePick32` control dispatches when the user
    // picks a different date. The map is parallel to
    // `disp_info_handlers` (both store a
    // `Box<dyn FnMut(isize)>` keyed by the control id) but is
    // a *third* independent map, distinct from
    // `notify_handlers` and `disp_info_handlers` — the
    // frame's `WM_NOTIFY` arm routes `DTN_DATETIMECHANGE`
    // into it via a dedicated `else if` branch.

    /// Registering a dtn handler must insert an entry into
    /// the dtn handler map keyed by the supplied control id.
    #[test]
    fn register_dtn_handler_stores_entry() {
        let f = Frame::for_testing();
        f.register_dtn_handler(0x5001, Box::new(|_lparam| {}));
        assert!(f.inner.borrow().dtn_handlers.contains_key(&0x5001));
    }

    /// Re-registering a dtn handler for the same id must
    /// replace the previous one (dropping the old
    /// `Box<dyn FnMut>`). This matches the
    /// `on_date_change` "one owner" model: a second
    /// `on_date_change` call for the same control id
    /// silently shadows the first.
    #[test]
    fn register_dtn_handler_replaces_previous() {
        let f = Frame::for_testing();
        f.register_dtn_handler(0x5002, Box::new(|_| {}));
        f.register_dtn_handler(0x5002, Box::new(|_| {}));
        // Only one entry remains for that id
        // (HashMap::insert is upsert-by-key).
        assert_eq!(f.inner.borrow().dtn_handlers.len(), 1);
        assert!(f.inner.borrow().dtn_handlers.contains_key(&0x5002));
    }

    /// Pin the public method's signature. A future change
    /// (e.g. a different return type or a borrowed
    /// parameter) would fail to compile here.
    #[test]
    #[allow(clippy::type_complexity)]
    fn signature_register_dtn_handler() {
        let _: fn(&Frame, u16, Box<dyn FnMut(isize)>) = Frame::register_dtn_handler;
    }

    /// The dtn handler closure must be able to capture
    /// local state — the date-picker `on_date_change`
    /// closure needs to capture the model cell the user
    /// wants to write into.
    #[test]
    fn dtn_handler_accepts_capturing_closure() {
        use std::cell::Cell;
        use std::rc::Rc;
        let f = Frame::for_testing();
        let count = Rc::new(Cell::new(0_u32));
        let count_for_closure = count.clone();
        f.register_dtn_handler(
            0x5003,
            Box::new(move |_lparam| {
                count_for_closure.set(count_for_closure.get() + 1);
            }),
        );
        // Direct invocation (no real Win32 message pump) —
        // just prove the closure was stored and can be
        // re-borrowed, mirroring the remove/insert pattern
        // used in the wndproc dispatch.
        let mut h = f.inner.borrow_mut().dtn_handlers.remove(&0x5003).unwrap();
        h(0);
        h(0);
        f.inner.borrow_mut().dtn_handlers.insert(0x5003, h);
        assert_eq!(count.get(), 2);
    }

    /// The three handler maps (`notify_handlers`,
    /// `disp_info_handlers`, `dtn_handlers`) must all be
    /// independent. The `WM_NOTIFY` arm in `frame_wnd_proc`
    /// uses each map for a *different* notification code,
    /// and a register on one map must never leak into
    /// another.
    #[test]
    fn notify_disp_info_and_dtn_maps_are_independent() {
        let f = Frame::for_testing();
        f.register_notify_handler(0x6001, Box::new(|_| {}));
        f.register_disp_info_handler(0x6001, Box::new(|_| {}));
        f.register_dtn_handler(0x6001, Box::new(|_| {}));
        assert!(f.inner.borrow().notify_handlers.contains_key(&0x6001));
        assert!(f.inner.borrow().disp_info_handlers.contains_key(&0x6001));
        assert!(f.inner.borrow().dtn_handlers.contains_key(&0x6001));
        assert_eq!(f.inner.borrow().notify_handlers.len(), 1);
        assert_eq!(f.inner.borrow().disp_info_handlers.len(), 1);
        assert_eq!(f.inner.borrow().dtn_handlers.len(), 1);
    }

    // ── OLE COM drop target (v0.5.8) ────────────────────────────
    //
    // The frame's `set_ole_drop_callback` method is the
    // integration point with the OLE COM `IDropTarget`
    // implementation in `crate::ole_dnd`. These tests pin the
    // registration contract: the entry is stored on success,
    // re-registration replaces, the closure captures state, the
    // signature matches, and the slot is independent from the
    // Shell-level `drop_files_handler` map.
    //
    // The actual `IDropTarget::*` vtable dispatch lives in
    // `ole_dnd::win` and is out of scope for these unit tests
    // (it requires a real `HWND` and a real `IDataObject`).
    // The two formats (CF_HDROP → Files, CF_UNICODETEXT → Text)
    // are exercised at the type level in `ole_dnd::tests`.

    /// `set_ole_drop_callback` must store a target in the
    /// `ole_drop_target` slot of `FrameData` after a successful
    /// registration. On a `null` `HWND` (the `for_testing`
    /// case on Windows) the Win32 `RegisterDragDrop` call
    /// returns an error, so on Windows we assert that the
    /// error is `RegisterFailed` and the slot stays empty; on
    /// non-Windows hosts `register` is a no-op stub that
    /// always succeeds, so the slot must be populated.
    #[test]
    fn set_ole_drop_callback_registers_or_fails_on_null_hwnd() {
        let f = Frame::for_testing();
        assert!(f.inner.borrow().ole_drop_target.is_none());
        let r = f.set_ole_drop_callback(|_data, _pos| {});
        #[cfg(target_os = "windows")]
        {
            // Null HWND → Win32 returns a non-zero HRESULT.
            let err = r.expect_err("null HWND must fail on Windows");
            assert!(
                matches!(err, OleDropError::RegisterFailed(_)),
                "expected RegisterFailed, got {:?}",
                err
            );
            assert!(f.inner.borrow().ole_drop_target.is_none());
        }
        #[cfg(not(target_os = "windows"))]
        {
            r.expect("non-Windows register must be a no-op Ok");
            assert!(f.inner.borrow().ole_drop_target.is_some());
        }
    }

    /// Re-registering an OLE drop callback for the same
    /// frame must replace the previous target. The
    /// `Option<OleDropTarget>` slot always holds at most
    /// one target. (Non-Windows only — a null HWND on
    /// Windows can't reach this path.)
    #[test]
    #[cfg(not(target_os = "windows"))]
    fn set_ole_drop_callback_replaces_previous() {
        let f = Frame::for_testing();
        f.set_ole_drop_callback(|_d, _p| {}).unwrap();
        assert!(f.inner.borrow().ole_drop_target.is_some());
        f.set_ole_drop_callback(|_d, _p| {}).unwrap();
        // The slot is single-valued; a second call overwrites
        // the first (the old `OleDropTarget`'s `Drop` impl
        // runs, but we don't observe it here).
        assert!(f.inner.borrow().ole_drop_target.is_some());
    }

    /// Pin the public method's signature. A future change
    /// (e.g. a different return type or a borrowed
    /// parameter) would fail to compile here. The
    /// `type_complexity` allow is for the `Box<dyn FnMut(...)>`
    /// return-position closure.
    #[test]
    #[allow(clippy::type_complexity)]
    fn signature_set_ole_drop_callback() {
        let _: fn(
            &Frame,
            Box<dyn FnMut(OleDroppedData, OleDropPosition)>,
        ) -> Result<(), OleDropError> = Frame::set_ole_drop_callback;
    }

    /// The OLE drop callback closure must be able to capture
    /// local state. The user typically wants to push the
    /// dropped paths/text into a model cell owned outside
    /// the frame.
    #[test]
    #[cfg(not(target_os = "windows"))]
    fn set_ole_drop_callback_accepts_capturing_closure() {
        use std::cell::Cell;
        use std::rc::Rc;
        let f = Frame::for_testing();
        let count = Rc::new(Cell::new(0_u32));
        let count_for_closure = count.clone();
        f.set_ole_drop_callback(move |_d, _p| {
            count_for_closure.set(count_for_closure.get() + 1);
        })
        .unwrap();
        // The closure is stored but never invoked (no COM
        // vtable dispatch happened). Re-registering a
        // *second* capturing closure must not panic.
        let count_for_closure_2 = count.clone();
        f.set_ole_drop_callback(move |_d, _p| {
            count_for_closure_2.set(count_for_closure_2.get() + 1);
        })
        .unwrap();
        // `count` is still 0 — the closures were stored but
        // never invoked (no COM vtable dispatch happened).
        assert_eq!(count.get(), 0);
    }

    /// The OLE drop slot is independent from the Shell-level
    /// `drop_files_handler` slot. The two protocols are
    /// documented to coexist (a frame may register both), so
    /// writing to one must not affect the other.
    #[test]
    fn ole_drop_target_and_drop_files_handler_are_independent() {
        let f = Frame::for_testing();
        f.set_drop_files_callback(|_| {});
        assert!(f.inner.borrow().drop_files_handler.is_some());
        assert!(f.inner.borrow().ole_drop_target.is_none());
        // (On Windows the OLE call would fail on a null HWND
        // — the independence of the slots is platform-agnostic
        // and is the property we care about here.)
    }

    /// Calling `set_ole_drop_callback` *after*
    /// `set_drop_files_callback` must keep the Shell-level
    /// handler in place. The two callbacks are orthogonal
    /// (different protocols, different fields in `FrameData`)
    /// and a write to the OLE slot must not clear the Shell
    /// slot.
    #[test]
    #[cfg(not(target_os = "windows"))]
    fn ole_and_shell_drops_coexist() {
        let f = Frame::for_testing();
        f.set_drop_files_callback(|_| {});
        f.set_ole_drop_callback(|_d, _p| {}).unwrap();
        assert!(f.inner.borrow().drop_files_handler.is_some());
        assert!(f.inner.borrow().ole_drop_target.is_some());
        // And the reverse ordering works too.
        let g = Frame::for_testing();
        g.set_ole_drop_callback(|_d, _p| {}).unwrap();
        g.set_drop_files_callback(|_| {});
        assert!(g.inner.borrow().drop_files_handler.is_some());
        assert!(g.inner.borrow().ole_drop_target.is_some());
    }
}
