//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! wxToolBar — an icon toolbar with optional separators and labels.
//!
//! On Windows this wraps the standard `ToolbarWindow32` common control.
//! Buttons are added with [`ToolBar::add_tool`]; use
//! [`ToolBar::add_separator`] to add a vertical gap. The image list is
//! attached with [`ToolBar::set_image_list`] (pass a reference to a
//! shared `ImageList`).
//!
//! After all buttons are added, call [`ToolBar::realize`] to commit the
//! layout. Tool clicks are delivered via `WM_COMMAND` and dispatched
//! through [`ToolBar::on_tool_clicked`].

use std::cell::RefCell;
use std::rc::Rc;

use crate::core::geometry::Rect;
use crate::core::widget::Widget;
use crate::dc::image_list::ImageList;
use crate::window::frame::Frame;

#[cfg(target_os = "windows")]
use crate::platform::as_hwnd;
use crate::platform::next_control_id;
#[cfg(target_os = "windows")]
use crate::platform::win32::to_wide;
#[cfg(target_os = "windows")]
use windows_sys::Win32::Foundation::*;
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::WindowsAndMessaging::*;

// ── Win32 ToolBar constants ─────────────────────────────────────────────

#[cfg(target_os = "windows")]
const TB_BUTTONSTRUCTSIZE: u32 = 0x041E;
#[cfg(target_os = "windows")]
const TB_SETBITMAPSIZE: u32 = 0x0420;
#[cfg(target_os = "windows")]
const TB_SETIMAGELIST: u32 = 0x0430;
#[cfg(target_os = "windows")]
const TB_ADDBUTTONS: u32 = 0x0444;
#[cfg(target_os = "windows")]
const TB_AUTOSIZE: u32 = 0x0421;
#[cfg(target_os = "windows")]
const TB_SETEXTENDEDSTYLE: u32 = 0x0454;
#[cfg(target_os = "windows")]
const TBSTYLE_EX_DRAWDDARROWS: u32 = 0x0000_0001;
#[cfg(target_os = "windows")]
const TBSTYLE_FLAT: u32 = 0x0800;
#[cfg(target_os = "windows")]
const TBSTYLE_TOOLTIPS: u32 = 0x0100;
/// Impedisce al controllo di agganciarsi da solo al bordo del parent.
///
/// `ToolbarWindow32` assume `CCS_TOP` quando non riceve alcuno stile di
/// allineamento: in quello stato il controllo si posiziona in alto a
/// sinistra nell'area client del parent e si stira alla sua larghezza,
/// ignorando `MoveWindow`. Servendoci di uno sizer per il layout, la
/// posizione deve restare sotto il nostro controllo.
#[cfg(target_os = "windows")]
const CCS_NOPARENTALIGN: u32 = 0x0008;
#[cfg(target_os = "windows")]
#[allow(dead_code)] // Win32 ABI surface — alternative text-only layout
const TBSTYLE_LIST: u32 = 0x1000;
#[cfg(target_os = "windows")]
#[allow(dead_code)] // Win32 ABI surface — transparent toolbar background
const TBSTYLE_TRANSPARENT: u32 = 0x8000;

#[cfg(target_os = "windows")]
const TBSTYLE_BUTTON: u8 = 0x00;
#[cfg(target_os = "windows")]
const TBSTYLE_SEP: u8 = 0x01;
/// `TBSTYLE_CHECK` — a dual-state button that toggles between the
/// pressed and non-pressed states each time the user clicks it.
#[cfg(target_os = "windows")]
const TBSTYLE_CHECK: u8 = 0x02;

#[cfg(target_os = "windows")]
const TBSTATE_ENABLED: u8 = 0x04;

/// `iString == -1` makes the toolbar request button text on demand via
/// `TTN_GETDISPINFO` (answered by the frame through
/// [`Frame::register_ttn_dispinfo_handler`]).
#[cfg(target_os = "windows")]
const LPSTR_TEXTCALLBACK: isize = -1;
/// `TTN_GETDISPINFOA` — `TTN_FIRST - 0` (ANSI text request).
#[cfg(target_os = "windows")]
const TTN_GETDISPINFOA: i32 = -520;
/// `TTN_GETDISPINFOW` — `TTN_FIRST - 10` (wide text request).
#[cfg(target_os = "windows")]
const TTN_GETDISPINFOW: i32 = -530;

/// Local mirror of `NMHDR` (the fields we read during `TTN_GETDISPINFO`).
#[cfg(target_os = "windows")]
#[repr(C)]
#[allow(dead_code)]
struct NmHdr {
    hwnd_from: HWND,
    id_from: usize,
    code: i32,
}

/// Local mirror of `NMTTDISPINFOW`. Only `hdr`, `lpsz_text` and
/// `sz_text` are read; the remaining fields exist purely to reproduce
/// the C struct layout so the pointer offsets line up.
#[cfg(target_os = "windows")]
#[repr(C)]
#[allow(dead_code)]
struct NmTtDispInfoW {
    hdr: NmHdr,
    lpsz_text: *mut u16,
    sz_text: [u16; 80],
    hinst: HINSTANCE,
    u_flags: u32,
    l_param: isize,
}

/// A TBBUTTON structure (12 bytes on 32-bit, 32 on 64-bit). We define
/// a layout-compatible 32-byte struct for x86_64.
#[cfg(target_os = "windows")]
#[repr(C)]
#[derive(Debug, Clone, Copy)]
#[allow(clippy::upper_case_acronyms)]
struct TBBUTTON {
    i_bitmap: i32,
    id_command: u32,
    fs_state: u8,
    fs_style: u8,
    _pad: u16,
    dw_data: usize,
    i_string: isize,
}

#[cfg(target_os = "windows")]
impl TBBUTTON {
    fn separator() -> Self {
        TBBUTTON {
            i_bitmap: 0,
            id_command: 0,
            fs_state: 0,
            fs_style: TBSTYLE_SEP,
            _pad: 0,
            dw_data: 0,
            i_string: 0,
        }
    }
}

// ── Inner type ─────────────────────────────────────────────────────────

struct ToolBarInner {
    #[cfg(target_os = "windows")]
    hwnd: HWND,
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    stub_handle: crate::platform::NativeHandle,
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    parent_handle: crate::platform::NativeHandle,
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    frame_handle: crate::platform::NativeHandle,
    /// Win32 control id (also used as the WM_COMMAND id / HMENU).
    control_id: u16,
    /// Buffered button specs (separators + real buttons) that are sent
    /// to the control on `realize()`.
    buttons: Vec<ToolSpec>,
    rect: Rect,
    visible: bool,
}

/// One logical entry on the toolbar.
#[derive(Clone)]
enum ToolSpec {
    Separator,
    Tool {
        /// Win32 control id (also used as the WM_COMMAND id).
        id: u16,
        /// Index into the attached image list, or -1 for no image.
        image_index: i32,
        /// Human-readable label, shown as the hover tooltip.
        label: String,
    },
    /// A dual-state (toggle) button, rendered with `TBSTYLE_CHECK`.
    Check {
        /// Win32 control id (also used as the WM_COMMAND id).
        id: u16,
        /// Index into the attached image list, or -1 for no image.
        image_index: i32,
        /// Human-readable label, shown as the hover tooltip.
        label: String,
    },
}

#[derive(Clone)]
pub struct ToolBar {
    inner: Rc<RefCell<ToolBarInner>>,
}

impl ToolBar {
    /// Create a new tool bar and attach it to the top of `frame`.
    pub fn new(frame: &Frame) -> Self {
        let control_id = next_control_id();

        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        let hwnd = unsafe {
            let parent = frame.hwnd();
            let wide_class = to_wide("ToolbarWindow32");
            CreateWindowExW(
                0,
                wide_class.as_ptr(),
                std::ptr::null(),
                WS_CHILD | WS_VISIBLE | TBSTYLE_FLAT | TBSTYLE_TOOLTIPS | CCS_NOPARENTALIGN,
                0,
                0,
                0,
                0,
                as_hwnd(parent),
                control_id as usize as HMENU,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        };

        #[cfg(all(
            not(target_os = "windows"),
            not(target_os = "macos"),
            not(target_os = "linux")
        ))]
        let _ = frame;

        #[cfg(any(target_os = "macos", target_os = "linux"))]
        let parent_handle = crate::platform::window_handle(frame);
        #[cfg(target_os = "macos")]
        let stub_handle = crate::platform::appkit::create_toolbar_strip(
            parent_handle,
            parent_handle,
            false,
            0,
            28,
        );
        #[cfg(target_os = "linux")]
        let stub_handle =
            crate::platform::gtk::create_toolbar_strip(parent_handle, parent_handle, false, 0, 28);

        ToolBar {
            inner: Rc::new(RefCell::new(ToolBarInner {
                #[cfg(target_os = "windows")]
                hwnd,
                #[cfg(any(target_os = "macos", target_os = "linux"))]
                stub_handle,
                #[cfg(any(target_os = "macos", target_os = "linux"))]
                parent_handle,
                #[cfg(any(target_os = "macos", target_os = "linux"))]
                frame_handle: parent_handle,
                control_id,
                buttons: Vec::new(),
                rect: Rect::new(0, 0, 0, 0),
                visible: true,
            })),
        }
    }

    /// Attach an image list. Must be called before [`ToolBar::realize`].
    #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
    pub fn set_image_list(&self, _image_list: &ImageList) {}

    #[cfg(target_os = "macos")]
    pub fn set_image_list(&self, image_list: &ImageList) {
        crate::platform::appkit::toolbar_set_image_list(
            self.inner.borrow().stub_handle,
            image_list.handle(),
            image_list.width(),
            image_list.height(),
        );
    }

    /// Attach an image list. Must be called before [`ToolBar::realize`].
    #[cfg(target_os = "windows")]
    pub fn set_image_list(&self, image_list: &ImageList) {
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            let state = self.inner.borrow();
            SendMessageW(state.hwnd, TB_SETIMAGELIST, 0, image_list.handle());
            // Match bitmap size to image list's image size
            let w = image_list.width();
            let h = image_list.height();
            let lparam = ((w as u32) & 0xFFFF) | (((h as u32) & 0xFFFF) << 16);
            SendMessageW(state.hwnd, TB_SETBITMAPSIZE, 0, lparam as isize);
        }
    }

    /// Add a tool button. The `image_index` is the index into the
    /// previously-attached image list. The `label` is the human-readable
    /// name of the tool, used as its hover tooltip (see
    /// [`ToolBar::enable_tooltips`]).
    /// Add a tool button with the given `id`, hover `label`, and
    /// `image_index` (from the attached [`crate::ImageList`]).
    ///
    /// The tooltip is only rendered when the tooltips are enabled (see
    /// [`ToolBar::enable_tooltips`]).
    pub fn add_tool(&self, id: u16, label: &str, image_index: i32) {
        #[cfg(target_os = "macos")]
        {
            crate::platform::appkit::toolbar_add_tool_with_tooltip(
                self.inner.borrow().stub_handle,
                id,
                image_index,
                label,
            );
        }
        #[cfg(target_os = "linux")]
        {
            crate::platform::gtk::toolbar_add_tool(
                self.inner.borrow().stub_handle,
                id,
                label,
                false,
            );
        }
        let mut state = self.inner.borrow_mut();
        state.buttons.push(ToolSpec::Tool {
            id,
            image_index,
            label: label.to_string(),
        });
    }

    /// Add a dual-state toggle button (rendered with `TBSTYLE_CHECK`).
    ///
    /// Like [`ToolBar::add_tool`], the button uses `image_index` from the
    /// attached image list and `label` as its hover tooltip. The control
    /// toggles the pressed/checked visual state automatically on each
    /// click; the click itself is still delivered through
    /// [`ToolBar::on_tool_clicked`].
    pub fn add_check_tool(&self, id: u16, label: &str, image_index: i32) {
        #[cfg(target_os = "macos")]
        {
            crate::platform::appkit::toolbar_add_tool_with_tooltip(
                self.inner.borrow().stub_handle,
                id,
                image_index,
                label,
            );
        }
        #[cfg(target_os = "linux")]
        {
            crate::platform::gtk::toolbar_add_tool(
                self.inner.borrow().stub_handle,
                id,
                label,
                true,
            );
        }
        let mut state = self.inner.borrow_mut();
        state.buttons.push(ToolSpec::Check {
            id,
            image_index,
            label: label.to_string(),
        });
    }

    /// Add a vertical separator.
    pub fn add_separator(&self) {
        #[cfg(target_os = "macos")]
        {
            crate::platform::appkit::toolbar_add_separator(self.inner.borrow().stub_handle);
        }
        #[cfg(target_os = "linux")]
        {
            crate::platform::gtk::toolbar_add_separator(self.inner.borrow().stub_handle);
        }
        self.inner.borrow_mut().buttons.push(ToolSpec::Separator);
    }

    /// Commit the buffered buttons to the control. Call this once after
    /// all tools / separators have been added and the image list has
    /// been attached. Calling it again is a no-op (already-realized
    /// buttons are cleared first).
    pub fn realize(&self) {
        #[cfg(target_os = "windows")]
        {
            let state = self.inner.borrow_mut();
            // First, instruct the control about the TBBUTTON size
            // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
            unsafe {
                SendMessageW(
                    state.hwnd,
                    TB_BUTTONSTRUCTSIZE,
                    std::mem::size_of::<TBBUTTON>() as usize,
                    0,
                );
                // Enable flat + transparent styles
                SendMessageW(
                    state.hwnd,
                    TB_SETEXTENDEDSTYLE,
                    0,
                    TBSTYLE_EX_DRAWDDARROWS as isize,
                );
                // Build the button array
                let mut btns: Vec<TBBUTTON> = Vec::with_capacity(state.buttons.len());
                for spec in state.buttons.iter() {
                    match spec {
                        ToolSpec::Separator => btns.push(TBBUTTON::separator()),
                        ToolSpec::Tool {
                            id, image_index, ..
                        } => btns.push(TBBUTTON {
                            i_bitmap: *image_index,
                            id_command: *id as u32,
                            fs_state: TBSTATE_ENABLED,
                            fs_style: TBSTYLE_BUTTON,
                            _pad: 0,
                            dw_data: 0,
                            // Request the tooltip text on demand via
                            // `TTN_GETDISPINFO` (answered by the frame).
                            i_string: LPSTR_TEXTCALLBACK,
                        }),
                        ToolSpec::Check {
                            id, image_index, ..
                        } => btns.push(TBBUTTON {
                            i_bitmap: *image_index,
                            id_command: *id as u32,
                            fs_state: TBSTATE_ENABLED,
                            fs_style: TBSTYLE_CHECK,
                            _pad: 0,
                            dw_data: 0,
                            i_string: LPSTR_TEXTCALLBACK,
                        }),
                    }
                }
                SendMessageW(
                    state.hwnd,
                    TB_ADDBUTTONS,
                    btns.len(),
                    btns.as_ptr() as isize,
                );
                // Force the toolbar to lay itself out
                SendMessageW(state.hwnd, TB_AUTOSIZE, 0, 0);
            }
        }
        #[cfg(target_os = "macos")]
        {
            let stub = self.inner.borrow().stub_handle;
            // Disable native NSToolbar – ToolBar is used as a subview
            // inside RibbonBar / other containers, not in the title bar.
            crate::platform::appkit::toolbar_set_prefer_native(stub, false);
            crate::platform::appkit::toolbar_realize(stub);
        }
        #[cfg(target_os = "linux")]
        {
            // GTK4 non ha GtkToolbar: il backend costruisce i pulsanti
            // dentro la `GtkBox` della striscia.
            crate::platform::gtk::toolbar_realize(self.inner.borrow().stub_handle);
        }
    }

    /// Register the hover tooltips for every button on this toolbar.
    ///
    /// Call once after [`ToolBar::realize`]. The tooltip shown on hover
    /// is the button's `label` (passed to [`ToolBar::add_tool`]).
    pub fn enable_tooltips(&self, frame: &Frame) {
        #[cfg(target_os = "windows")]
        {
            let inner = self.inner.clone();
            let control_id = self.inner.borrow().control_id;
            frame.register_ttn_dispinfo_handler(
                control_id,
                Box::new(move |lparam| {
                    let inner = inner.borrow();
                    // SAFETY: Win32 passes a live `NMTTDISPINFOW` pointer.
                    unsafe { fill_tooltip_text(lparam, &inner.buttons) };
                }),
            );
        }
        #[cfg(not(target_os = "windows"))]
        {
            let _ = frame;
        }
    }

    /// Register a callback that fires when any of the tools on this
    /// toolbar is clicked. The callback receives the id of the tool.
    pub fn on_tool_clicked<F: FnMut(u16) + 'static>(&self, frame: &Frame, callback: F) {
        #[cfg(target_os = "windows")]
        {
            // Share a single FnMut across all per-id handlers via
            // Rc<RefCell<...>>. We can't simply move `callback` into
            // the first closure because we need to clone it for every
            // registered tool id.
            let callback = std::rc::Rc::new(std::cell::RefCell::new(callback));

            // Collect all tool ids registered with the toolbar
            let tool_ids: Vec<u16> = self
                .inner
                .borrow()
                .buttons
                .iter()
                .filter_map(|s| match s {
                    ToolSpec::Tool { id, .. } | ToolSpec::Check { id, .. } => Some(*id),
                    _ => None,
                })
                .collect();

            for id in tool_ids {
                let cb = callback.clone();
                frame.register_command_handler(id, Box::new(move || cb.borrow_mut()(id)));
            }
        }
        #[cfg(all(
            not(target_os = "windows"),
            not(target_os = "macos"),
            not(target_os = "linux")
        ))]
        {
            let _ = (frame, callback);
        }
        // macOS e Linux condividono lo stesso percorso: i bottoni
        // inoltrano il comando alla frame e qui registriamo l'handler.
        #[cfg(any(target_os = "macos", target_os = "linux"))]
        {
            let callback = std::rc::Rc::new(std::cell::RefCell::new(callback));
            let tool_ids: Vec<u16> = self
                .inner
                .borrow()
                .buttons
                .iter()
                .filter_map(|s| match s {
                    ToolSpec::Tool { id, .. } | ToolSpec::Check { id, .. } => Some(*id),
                    _ => None,
                })
                .collect();
            for id in tool_ids {
                let cb = callback.clone();
                frame.register_command_handler(id, Box::new(move || cb.borrow_mut()(id)));
            }
        }
    }

    /// Return the native window handle.
    #[cfg(target_os = "windows")]
    pub fn hwnd(&self) -> HWND {
        self.inner.borrow().hwnd
    }

    /// Return the native window handle (macOS: the AppKit toolbar strip
    /// handle; Linux: the `GtkBox` strip handle — same value used by the
    /// other toolbar helpers).
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    pub fn hwnd(&self) -> crate::platform::NativeHandle {
        self.inner.borrow().stub_handle
    }

    /// Borrow the underlying [`Widget`] state (layout / visibility).
    pub fn as_widget_ref(&self) -> crate::core::widget::WidgetRef {
        self.inner.clone()
    }
}

// ── Widget trait ───────────────────────────────────────────────────────

impl Widget for ToolBarInner {
    fn native_handle(&self) -> isize {
        #[cfg(target_os = "windows")]
        {
            self.hwnd as isize
        }
        #[cfg(target_os = "macos")]
        {
            self.stub_handle
        }
        #[cfg(target_os = "linux")]
        {
            self.stub_handle
        }
        #[cfg(all(
            not(target_os = "windows"),
            not(target_os = "macos"),
            not(target_os = "linux")
        ))]
        {
            0
        }
    }

    fn set_position(&mut self, x: i32, y: i32) {
        self.rect.x = x;
        self.rect.y = y;
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            MoveWindow(
                self.hwnd,
                x,
                y,
                self.rect.width as i32,
                self.rect.height as i32,
                1,
            );
        }
        #[cfg(target_os = "macos")]
        {
            crate::platform::appkit::set_view_frame(
                self.parent_handle,
                self.stub_handle,
                self.rect,
            );
        }
        #[cfg(target_os = "linux")]
        {
            crate::platform::gtk::set_view_frame(self.parent_handle, self.stub_handle, self.rect);
        }
    }

    fn set_size(&mut self, w: u32, h: u32) {
        self.rect.width = w;
        self.rect.height = h;
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            MoveWindow(self.hwnd, self.rect.x, self.rect.y, w as i32, h as i32, 1);
        }
        #[cfg(target_os = "macos")]
        {
            crate::platform::appkit::set_view_frame(
                self.parent_handle,
                self.stub_handle,
                self.rect,
            );
        }
        #[cfg(target_os = "linux")]
        {
            crate::platform::gtk::set_view_frame(self.parent_handle, self.stub_handle, self.rect);
        }
    }

    fn rect(&self) -> Rect {
        self.rect
    }

    fn is_visible(&self) -> bool {
        self.visible
    }

    fn set_visible(&mut self, visible: bool) {
        self.visible = visible;
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            ShowWindow(self.hwnd, if visible { SW_SHOW } else { SW_HIDE });
        }
        #[cfg(target_os = "macos")]
        {
            crate::platform::appkit::set_view_visible(self.stub_handle, visible);
        }
        #[cfg(target_os = "linux")]
        {
            crate::platform::gtk::set_view_visible(self.stub_handle, visible);
        }
    }

    fn is_enabled(&self) -> bool {
        true
    }

    fn set_enabled(&mut self, _enabled: bool) {
        // ToolBar has no enabled state at the bar level (each tool has
        // its own enabled state managed by the control).
    }
}

// ── Tooltip text helpers ──────────────────────────────────────────────

/// Look up the label of the button whose command id is `cmd_id`.
#[cfg(target_os = "windows")]
fn tooltip_label_for(buttons: &[ToolSpec], cmd_id: u32) -> Option<&str> {
    buttons.iter().find_map(|spec| match spec {
        ToolSpec::Tool { id, label, .. } | ToolSpec::Check { id, label, .. }
            if *id as u32 == cmd_id =>
        {
            Some(label.as_str())
        }
        _ => None,
    })
}

/// Fill the `TTN_GETDISPINFO` buffer (`NMTTDISPINFOW`) with the label of
/// the button requesting its tooltip.
#[cfg(target_os = "windows")]
unsafe fn fill_tooltip_text(lparam: isize, buttons: &[ToolSpec]) {
    let info = lparam as *mut NmTtDispInfoW;
    if info.is_null() {
        return;
    }
    // For a toolbar tool registered without `TTF_IDISHWND`, the button's
    // command id is reported in `NMHDR.idFrom` — NOT in the `uFlags`
    // field. Reading the wrong field looks up a non-existent tool and
    // leaves the tooltip empty.
    let cmd_id = (*info).hdr.id_from as u32;
    let Some(label) = tooltip_label_for(buttons, cmd_id) else {
        return;
    };
    // `szText` starts at the same offset in NMTTDISPINFO**A** and
    // NMTTDISPINFO**W**; only the element width differs. Pick the width
    // from the notification code so the control reads back the right
    // string instead of stopping at the first embedded NUL byte (which
    // is what produces the "single character" tooltip).
    let sz = std::ptr::addr_of_mut!((*info).sz_text);
    if (*info).hdr.code == TTN_GETDISPINFOA {
        // ANSI: write one byte per character.
        let bytes: Vec<u8> = label.chars().map(|c| c as u32 as u8).collect();
        let copy_len = bytes.len().min(79);
        let dst = sz.cast::<u8>();
        for (i, b) in bytes.iter().take(copy_len).enumerate() {
            *dst.add(i) = *b;
        }
        *dst.add(copy_len) = 0;
        (*info).lpsz_text = dst.cast::<u16>();
    } else {
        // Wide: write one UTF-16 code unit per character.
        let wide = to_wide(label);
        let copy_len = wide.len().saturating_sub(1).min(79);
        let dst = sz.cast::<u16>();
        for (i, w) in wide.iter().take(copy_len).enumerate() {
            *dst.add(i) = *w;
        }
        *dst.add(copy_len) = 0;
        (*info).lpsz_text = dst;
    }
}
