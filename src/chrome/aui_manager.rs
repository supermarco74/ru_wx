//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Advanced UI docking (`wxAuiManager` family).
//!
//! # Modules
//!
//! * [`AuiManager`] — the main docking manager; owns a list of
//!   [`AuiPaneInfo`] entries and re-positions them on
//!   [`AuiManager::update`].
//! * [`AuiPaneInfo`] — descriptor for a single pane: name, caption,
//!   dock side, best / min / max size, layer, row, caption buttons.
//! * [`AuiDockArt`] — the theme settings (sash width, colours,
//!   gradient type) used by the manager when laying out.
//! * [`AuiNotebook`] — a tabbed notebook used as the host for a
//!   notebook pane (a pane that hosts a stack of pages).
//! * [`AuiFloatingFrame`] — a real Win32 top-level popup window used
//!   to host a floating pane. Detached panes are re-docked when the
//!   floating window is closed or its title bar is double-clicked.
//!
//! # Win32 implementation
//!
//! [`AuiManager::update`] walks the registered panes once, computes
//! the rectangle of every docked side and the center pane, and
//! positions each pane's HWND with `MoveWindow`. The manager does
//! **not** draw a sash; the host application is expected to either
//! leave a fixed width / height for every dock side (using
//! [`AuiPaneInfo::with_best_size`]) or draw its own sashes above
//! [`AuiManager::update`]. The companion [`AuiFloatingFrame`] is a
//! real `WS_POPUP | WS_CAPTION | WS_THICKFRAME` top-level window
//! that hosts the detached pane and re-docks on close.

use crate::chrome::aui_tool_bar::AuiDockSide;
use crate::core::geometry::Rect;
use crate::core::widget::{Widget, WidgetRef};
use crate::window::frame::Frame;

#[cfg(target_os = "windows")]
use crate::core::dpi::get_dpi_for_window;
#[cfg(target_os = "windows")]
use crate::platform::win32::to_wide;
#[cfg(target_os = "windows")]
use crate::platform::{as_hwnd, NativeHandleExt};
#[cfg(target_os = "windows")]
use windows_sys::Win32::Foundation::RECT;
#[cfg(target_os = "windows")]
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
#[cfg(target_os = "windows")]
use windows_sys::Win32::Graphics::Gdi::{GetStockObject, HOLLOW_BRUSH};
#[cfg(target_os = "windows")]
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, GetClientRect, GetParent, GetWindowLongPtrW,
    MoveWindow, RegisterClassExW, SendMessageW, SetParent, SetWindowLongPtrW, ShowWindow,
    CS_HREDRAW, CS_VREDRAW, GWLP_USERDATA, GWLP_WNDPROC, HMENU, SW_HIDE, SW_SHOW, WM_COMMAND,
    WM_DESTROY, WM_HSCROLL, WM_NCLBUTTONDBLCLK, WM_NOTIFY, WM_VSCROLL, WNDCLASSEXW, WS_CAPTION,
    WS_CHILD, WS_POPUP, WS_SYSMENU, WS_THICKFRAME, WS_VISIBLE,
};

// ── Pane info ────────────────────────────────────────────────────

/// Caption button style for a pane.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuiPaneButton {
    Close,
    Maximize,
    Minimize,
    Pin,
    /// Custom button identified by a user-supplied id (the user is
    /// responsible for drawing and event handling).
    Custom(u16),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Gradient style used by the dock art.
pub enum AuiDockArtGradient {
    /// Solid colour.
    None,
    /// Vertical (top → bottom).
    Vertical,
    /// Horizontal (left → right).
    Horizontal,
}

/// Descriptor for a single AUI pane (`wxAuiPaneInfo`).
///
/// Construct a pane with one of the convenience constructors:
/// [`AuiPaneInfo::new`], [`AuiPaneInfo::toolbar`],
/// [`AuiPaneInfo::notebook`], or [`AuiPaneInfo::center`], then chain
/// the `with_*` setters to configure best / min size, caption text,
/// buttons, dock direction, etc.
#[derive(Debug, Clone)]
pub struct AuiPaneInfo {
    pub name: String,
    pub caption: String,
    /// Which edge of the frame the pane is docked to. Use
    /// [`AuiPaneInfo::with_dock_side`] to change.
    pub dock_side: AuiDockSide,
    /// `true` if the pane is currently floating. The
    /// [`AuiManager::update`] method skips floating panes when
    /// computing dock layout.
    pub floating: bool,
    /// `false` hides the pane HWND entirely (it is still in the
    /// manager's list and the dock rectangle is reserved for it).
    pub shown: bool,
    /// Native window handle (`Widget::native_handle`) positioned by
    /// [`AuiManager::update`].
    pub window_handle: Option<isize>,
    /// Preferred size when docked (width × height). The width is
    /// used for left / right panes, the height for top / bottom
    /// panes. Center panes use this as the default minimum size.
    pub best_size: (i32, i32),
    /// Minimum size. Pane HWNDs are never resized below this.
    pub min_size: (i32, i32),
    /// Maximum size (0, 0 means "unlimited").
    pub max_size: (i32, i32),
    /// Stacking layer — panes in a lower layer are placed closer to
    /// the edge, panes in a higher layer are pushed inward. The
    /// center pane sits on layer 0 by convention.
    pub layer: i32,
    /// Stacking row inside a layer — panes with row 0 are closest
    /// to the edge, higher rows are pushed toward the center. This
    /// is how multiple toolbars stack on the top edge.
    pub row: i32,
    /// Position offset along the dock edge.
    pub position: i32,
    /// `true` if the pane shows a caption bar.
    pub caption_visible: bool,
    /// `true` if the pane shows a small gripper on the leading
    /// edge (only meaningful for top / bottom panes).
    pub gripper: bool,
    /// `true` if the user can detach the pane by dragging its
    /// caption. When [`AuiPaneInfo::pinned`] is `true`, drag-to-float
    /// is blocked until the user clicks the pin button.
    pub floatable: bool,
    /// When `true`, the pane is locked in the dock and cannot be
    /// dragged to float. Toggled by the pin caption button.
    pub pinned: bool,
    /// `true` if the user can re-dock the pane to another edge.
    pub dockable: bool,
    /// `true` if the pane is treated as a toolbar pane (no caption,
    /// fixed height). Affects how the manager reserves space.
    pub is_toolbar: bool,
    /// Caption buttons. Each button takes ~14 px of caption width
    /// and is rendered as a glyph by the dock art.
    pub buttons: Vec<AuiPaneButton>,
    /// Cached rect from the last [`AuiManager::update`]. Useful for
    /// painting the caption bar / sash.
    pub rect: Rect,
    /// When `true`, the pane can collapse to a thin strip on its dock
    /// edge ([`Self::auto_hidden`]).
    pub auto_hide: bool,
    /// Runtime: pane collapsed to the auto-hide strip.
    pub auto_hidden: bool,
    /// Best size saved when entering auto-hide (restored on expand).
    saved_best_size: (i32, i32),
}

/// Interactive sash between a docked pane and the centre area.
#[derive(Clone, Debug)]
pub struct AuiSashSpec {
    pub pane_name: String,
    pub vertical: bool,
    pub rect: Rect,
    pub sash_color: u32,
}

const AUTO_HIDE_STRIP: i32 = 22;

impl AuiPaneInfo {
    /// Create a generic pane with the given name.
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            caption: name.to_string(),
            dock_side: AuiDockSide::Top,
            rect: Rect::new(0, 0, 0, 0),
            floating: false,
            shown: true,
            window_handle: None,
            best_size: (200, 200),
            min_size: (50, 50),
            max_size: (0, 0),
            layer: 0,
            row: 0,
            position: 0,
            caption_visible: true,
            gripper: true,
            floatable: true,
            pinned: true,
            dockable: true,
            is_toolbar: false,
            buttons: Vec::new(),
            auto_hide: false,
            auto_hidden: false,
            saved_best_size: (200, 200),
        }
    }

    /// Enable auto-hide: the pane can collapse to a thin strip on its
    /// dock edge until the user clicks the strip to expand it again.
    pub fn with_auto_hide(mut self, auto_hide: bool) -> Self {
        self.auto_hide = auto_hide;
        self
    }

    /// Create a pane that hosts a top-docked toolbar. Caption is
    /// hidden by default.
    pub fn toolbar(name: &str) -> Self {
        let mut p = Self::new(name);
        p.caption_visible = false;
        p.is_toolbar = true;
        p.best_size = (0, 32);
        p.min_size = (0, 28);
        p
    }

    /// Create a pane that hosts a top-docked toolbar and uses the
    /// given `height` as its best / min dock height.
    pub fn toolbar_with_height(name: &str, height: i32) -> Self {
        let mut p = Self::toolbar(name);
        p.best_size = (0, height);
        p.min_size = (0, height.max(28));
        p
    }

    /// Create a notebook pane. The pane itself is just a host
    /// rectangle; the actual tab control is built via
    /// [`AuiNotebook`] and its HWND should be passed via
    /// [`AuiPaneInfo::with_window`].
    pub fn notebook(name: &str) -> Self {
        let mut p = Self::new(name);
        p.best_size = (300, 200);
        p.min_size = (100, 80);
        p.caption_visible = true;
        p
    }

    /// Create the center pane. The center pane is always positioned
    /// last and fills whatever space is left after the docked panes
    /// have reserved their strips. The center pane should be the
    /// primary content area of the frame.
    pub fn center(name: &str) -> Self {
        let mut p = Self::new(name);
        p.caption_visible = false;
        p.gripper = false;
        p.dock_side = AuiDockSide::Floating; // sentinel: center is special-cased
        p.best_size = (400, 300);
        p.min_size = (50, 50);
        p
    }

    /// Create a left-docked side pane (e.g. a project tree).
    pub fn left(name: &str) -> Self {
        let mut p = Self::new(name);
        p.dock_side = AuiDockSide::Left;
        p.best_size = (200, 0);
        p.min_size = (100, 0);
        p
    }

    /// Create a right-docked side pane (e.g. a properties panel).
    pub fn right(name: &str) -> Self {
        let mut p = Self::new(name);
        p.dock_side = AuiDockSide::Right;
        p.best_size = (200, 0);
        p.min_size = (100, 0);
        p
    }

    /// Create a bottom-docked pane (e.g. an output / log pane).
    pub fn bottom(name: &str) -> Self {
        let mut p = Self::new(name);
        p.dock_side = AuiDockSide::Bottom;
        p.best_size = (0, 150);
        p.min_size = (0, 50);
        p
    }

    /// Set the caption text shown in the pane's caption bar.
    pub fn with_caption(mut self, caption: &str) -> Self {
        self.caption = caption.to_string();
        self
    }

    /// Attach a child widget HWND. The widget will be positioned by
    /// [`AuiManager::update`].
    pub fn with_window(mut self, widget: &dyn Widget) -> Self {
        self.window_handle = Some(widget.native_handle());
        self
    }

    /// Attach a child widget via a [`WidgetRef`] (the
    /// `Rc<RefCell<dyn Widget>>` returned by the public wrappers'
    /// `as_widget_ref()` method).
    ///
    /// This is the convenience overload used by the public wrappers
    /// ([`crate::Panel`], [`crate::StaticText`], [`crate::TextCtrl`],
    /// [`crate::AuiToolBar`], …) which do not implement [`Widget`]
    /// directly but expose a `WidgetRef` to their inner state.
    pub fn with_window_ref(mut self, widget_ref: &WidgetRef) -> Self {
        self.window_handle = Some(widget_ref.borrow().native_handle());
        self
    }

    /// Attach a raw HWND. Use this when the pane is not backed by a
    /// `Widget` you have a Rust handle to (e.g. an HWND returned by
    /// `Win32`).
    #[cfg(target_os = "windows")]
    pub fn with_hwnd(mut self, hwnd: HWND) -> Self {
        self.window_handle = Some(hwnd as isize);
        self
    }

    /// Set the dock side.
    pub fn with_dock_side(mut self, side: AuiDockSide) -> Self {
        self.dock_side = side;
        self
    }

    /// Set the preferred dock size (width × height).
    pub fn with_best_size(mut self, w: i32, h: i32) -> Self {
        self.best_size = (w.max(0), h.max(0));
        self
    }

    /// Set the minimum allowed size. The manager will not shrink
    /// the pane below this when re-laying out.
    pub fn with_min_size(mut self, w: i32, h: i32) -> Self {
        self.min_size = (w.max(0), h.max(0));
        self
    }

    /// Set the maximum allowed size. `(0, 0)` means "no limit" and
    /// is the default.
    pub fn with_max_size(mut self, w: i32, h: i32) -> Self {
        self.max_size = (w.max(0), h.max(0));
        self
    }

    /// Set the layer the pane belongs to. Panes in a lower layer
    /// sit closer to the frame edge.
    pub fn with_layer(mut self, layer: i32) -> Self {
        self.layer = layer.max(0);
        self
    }

    /// Set the row inside a layer. Rows are stacked from the edge
    /// inward; row 0 is the outermost.
    pub fn with_row(mut self, row: i32) -> Self {
        self.row = row.max(0);
        self
    }

    /// Set the position offset along the dock edge.
    pub fn with_position(mut self, pos: i32) -> Self {
        self.position = pos.max(0);
        self
    }

    /// Hide / show the caption bar.
    pub fn with_caption_visible(mut self, visible: bool) -> Self {
        self.caption_visible = visible;
        self
    }

    /// Hide / show the leading-edge gripper.
    pub fn with_gripper(mut self, gripper: bool) -> Self {
        self.gripper = gripper;
        self
    }

    /// Mark the pane as a toolbar pane (no caption, fixed height
    /// on top / bottom).
    pub fn with_is_toolbar(mut self, is_toolbar: bool) -> Self {
        self.is_toolbar = is_toolbar;
        self
    }

    /// Add a caption button (close, maximize, …). Multiple buttons
    /// can be added by calling this repeatedly; they are rendered
    /// left-to-right in the order they were added.
    pub fn with_button(mut self, button: AuiPaneButton) -> Self {
        self.buttons.push(button);
        self
    }

    /// Show the pane. Hidden panes keep their place in the dock
    /// layout but the HWND is hidden.
    pub fn show(&mut self) {
        self.shown = true;
    }

    /// Hide the pane. The HWND is hidden but the dock space is
    /// still reserved (set [`AuiPaneInfo::shown`] to `false` and
    /// call [`AuiManager::update`] to drop the reserved strip).
    pub fn hide(&mut self) {
        self.shown = false;
    }

    /// `true` if the pane is currently visible.
    pub fn is_shown(&self) -> bool {
        self.shown
    }

    /// `true` if the pane is currently floating.
    pub fn is_floating(&self) -> bool {
        self.floating
    }

    /// `true` if this pane is the special "center" pane (created
    /// with [`AuiPaneInfo::center`]). The center pane always fills
    /// whatever space is left after the docked side panes are
    /// positioned.
    pub fn is_center(&self) -> bool {
        // AuiDockSide::Floating is the sentinel value for center
        // panes set by `center()`. Detect it via the caption being
        // hidden and gripper being false.
        !self.caption_visible && !self.gripper && !self.is_toolbar && self.buttons.is_empty()
    }
}

// ── Dock art / theme ─────────────────────────────────────────────

/// Theme settings (`wxAuiDockArt`).
///
/// Holds the metrics and colours used by [`AuiManager`] when laying
/// out panes. The values are read by the manager's `update` method
/// to leave room for sashes and caption bars; the actual painting
/// of those decorations is left to the host application because the
/// manager does not draw owner-content.
#[derive(Debug, Clone)]
pub struct AuiDockArt {
    sash_width: u32,
    /// Width reserved for the caption bar of a docked pane. The
    /// pane HWND is positioned below this strip.
    caption_height: u32,
    /// Padding inserted between consecutive panes on the same dock
    /// edge.
    pane_padding: u32,
    /// Background colour of the area not covered by any pane.
    background: u32,
    /// Foreground / accent colour of the active caption.
    active_caption: u32,
    /// Foreground colour of an inactive caption.
    inactive_caption: u32,
    /// Sash colour.
    sash: u32,
    /// Border colour.
    border: u32,
    /// Gradient style used by the caption bar.
    gradient: AuiDockArtGradient,
}

impl Default for AuiDockArt {
    fn default() -> Self {
        Self::new()
    }
}

impl AuiDockArt {
    /// Metriche e colori predefiniti. La palette dipende dal tema di
    /// sistema (`Appearance::System`): con Windows in dark mode i
    /// default chiari rendevano caption illeggibili e stonate rispetto
    /// al resto dell'interfaccia.
    pub fn new() -> Self {
        if crate::core::appearance::Appearance::System.resolve() {
            Self {
                sash_width: 5,
                caption_height: 21,
                pane_padding: 4,
                background: 0x0020_2020,
                active_caption: 0x003E_3E42,
                inactive_caption: 0x002B_2B2B,
                sash: 0x004E_4E4E,
                border: 0x005A_5A5A,
                gradient: AuiDockArtGradient::Vertical,
            }
        } else {
            Self {
                sash_width: 5,
                caption_height: 21,
                pane_padding: 4,
                // RGB — `COLOR_3DFACE` / `COLOR_INACTIVECAPTION` etc.
                background: 0x00F0_F0F0,
                active_caption: 0x00D8_E9F8,
                inactive_caption: 0x00E8_E8E8,
                sash: 0x00C8_C8C8,
                border: 0x0080_8080,
                gradient: AuiDockArtGradient::Vertical,
            }
        }
    }

    pub fn sash_width(&self) -> u32 {
        self.sash_width
    }
    pub fn caption_height(&self) -> u32 {
        self.caption_height
    }
    pub fn pane_padding(&self) -> u32 {
        self.pane_padding
    }
    pub fn background(&self) -> u32 {
        self.background
    }
    pub fn active_caption(&self) -> u32 {
        self.active_caption
    }
    pub fn inactive_caption(&self) -> u32 {
        self.inactive_caption
    }
    pub fn sash(&self) -> u32 {
        self.sash
    }
    pub fn border(&self) -> u32 {
        self.border
    }
    pub fn gradient(&self) -> AuiDockArtGradient {
        self.gradient
    }

    pub fn set_sash_width(&mut self, w: u32) {
        self.sash_width = w.max(1);
    }
    pub fn set_caption_height(&mut self, h: u32) {
        self.caption_height = h.max(8);
    }
    pub fn set_pane_padding(&mut self, p: u32) {
        self.pane_padding = p;
    }
    pub fn set_background(&mut self, c: u32) {
        self.background = c;
    }
    pub fn set_active_caption(&mut self, c: u32) {
        self.active_caption = c;
    }
    pub fn set_inactive_caption(&mut self, c: u32) {
        self.inactive_caption = c;
    }
    pub fn set_sash(&mut self, c: u32) {
        self.sash = c;
    }
    pub fn set_border(&mut self, c: u32) {
        self.border = c;
    }
    pub fn set_gradient(&mut self, g: AuiDockArtGradient) {
        self.gradient = g;
    }
}

// ── Floating frame ───────────────────────────────────────────────

/// Floating top-level window used to host a detached pane.
///
/// Constructed with [`AuiFloatingFrame::new`]; the floating window
/// is positioned by [`AuiManager`] when a pane is detached and
/// destroyed when it is re-docked.
pub struct AuiFloatingFrame {
    title: String,
    rect: Rect,
    #[cfg(target_os = "windows")]
    hwnd: HWND,
    #[cfg(target_os = "macos")]
    frame_handle: crate::platform::NativeHandle,
}

#[cfg(target_os = "windows")]
static FLOATING_MANAGER_CLASS_REGISTERED: std::sync::OnceLock<()> = std::sync::OnceLock::new();

#[cfg(target_os = "windows")]
fn ensure_floating_manager_class() {
    FLOATING_MANAGER_CLASS_REGISTERED.get_or_init(|| {
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            let hinstance = GetModuleHandleW(std::ptr::null());
            let class_name = to_wide("RuWxAuiManagerFloating");
            let wc = WNDCLASSEXW {
                cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
                style: CS_HREDRAW | CS_VREDRAW,
                lpfnWndProc: Some(aui_manager_floating_wnd_proc),
                cbClsExtra: 0,
                cbWndExtra: 0,
                hInstance: hinstance,
                hIcon: std::ptr::null_mut(),
                hCursor: std::ptr::null_mut(),
                hbrBackground: GetStockObject(HOLLOW_BRUSH) as _,
                lpszMenuName: std::ptr::null(),
                lpszClassName: class_name.as_ptr(),
                hIconSm: std::ptr::null_mut(),
            };
            RegisterClassExW(&wc);
        }
    });
}

#[cfg(target_os = "windows")]
unsafe extern "system" fn aui_manager_floating_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: usize,
    lparam: isize,
) -> isize {
    match msg {
        WM_NCLBUTTONDBLCLK => {
            // Re-dock by destroying the floating window; the owner
            // of the floating frame is expected to listen for
            // WM_CLOSE and call back into the manager.
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }
        WM_DESTROY => {
            set_window_long_ptr_w_dummy(hwnd, GWLP_USERDATA, 0);
            0
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

#[cfg(target_os = "windows")]
unsafe fn set_window_long_ptr_w_dummy(hwnd: HWND, idx: i32, val: isize) {
    use windows_sys::Win32::UI::WindowsAndMessaging::SetWindowLongPtrW;
    SetWindowLongPtrW(hwnd, idx, val);
}

impl AuiFloatingFrame {
    /// Create a new floating frame. The frame is **not** shown
    /// automatically — call [`AuiFloatingFrame::show`] when ready.
    pub fn new(title: &str, rect: Rect) -> Self {
        Self {
            title: title.to_string(),
            rect,
            #[cfg(target_os = "windows")]
            hwnd: std::ptr::null_mut(),
            #[cfg(target_os = "macos")]
            frame_handle: 0,
        }
    }

    /// Show the floating window as a `WS_POPUP` top-level window
    /// with a caption and thick frame.
    #[cfg(target_os = "windows")]
    pub fn show(&mut self) {
        ensure_floating_manager_class();
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            let title_w = to_wide(&self.title);
            let h = CreateWindowExW(
                0,
                to_wide("RuWxAuiManagerFloating").as_ptr(),
                title_w.as_ptr(),
                WS_POPUP | WS_CAPTION | WS_THICKFRAME | WS_SYSMENU | WS_VISIBLE,
                self.rect.x,
                self.rect.y,
                self.rect.width.max(120) as i32,
                self.rect.height.max(80) as i32,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                GetModuleHandleW(std::ptr::null()),
                std::ptr::null_mut(),
            );
            self.hwnd = h;
        }
    }

    /// Hide and destroy the floating window. Subsequent calls to
    /// [`AuiFloatingFrame::show`] build a new window.
    #[cfg(target_os = "windows")]
    pub fn close(&mut self) {
        if !self.hwnd.is_null() {
            // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
            unsafe {
                DestroyWindow(self.hwnd);
            }
            self.hwnd = std::ptr::null_mut();
        }
    }

    /// Re-parent a child HWND into the floating window so the
    /// floating frame hosts it. If the floating window has not been
    /// shown yet it is created on-demand with the previously-supplied
    /// `rect` so the re-parent always succeeds. No-op on non-Windows.
    #[cfg(target_os = "windows")]
    pub fn host_child(&mut self, child_hwnd: isize) {
        if child_hwnd == 0 {
            return;
        }
        // Ensure the OS-level window exists before re-parenting. We
        // take `&mut self` so we can lazily create it here.
        if self.hwnd.is_null() {
            self.show();
        }
        let hwnd = self.hwnd;
        if hwnd.is_null() {
            return;
        }
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            SetParent(child_hwnd as HWND, hwnd);
            let mut r: RECT = std::mem::zeroed();
            GetClientRect(hwnd, &mut r);
            MoveWindow(
                child_hwnd as HWND,
                0,
                0,
                (r.right - r.left).max(1),
                (r.bottom - r.top).max(1),
                1,
            );
        }
    }

    /// Show the floating window (no-op on Linux stub).
    #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
    pub fn show(&mut self) {}

    #[cfg(target_os = "macos")]
    pub fn show(&mut self) {
        if self.frame_handle == 0 {
            self.frame_handle =
                crate::platform::appkit::create_floating_frame(&self.title, self.rect);
        }
        crate::platform::appkit::floating_frame_show(self.frame_handle);
    }

    /// Close the floating window (no-op on Linux stub).
    #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
    pub fn close(&mut self) {}

    #[cfg(target_os = "macos")]
    pub fn close(&mut self) {
        if self.frame_handle != 0 {
            crate::platform::appkit::floating_frame_close(self.frame_handle);
            self.frame_handle = 0;
        }
    }

    /// Re-parent a child HWND (no-op on Linux stub).
    #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
    pub fn host_child(&self, _child_hwnd: isize) {}

    #[cfg(target_os = "macos")]
    pub fn host_child(&mut self, child_hwnd: isize) {
        if child_hwnd == 0 {
            return;
        }
        if self.frame_handle == 0 {
            self.show();
        }
        if self.frame_handle != 0 {
            crate::platform::appkit::floating_frame_host_child(self.frame_handle, child_hwnd);
        }
    }

    /// Title text shown in the floating window's caption.
    pub fn title(&self) -> &str {
        &self.title
    }
    /// Last known rectangle of the floating window.
    pub fn rect(&self) -> Rect {
        self.rect
    }
}

// ── AuiNotebook ──────────────────────────────────────────────────

/// Tabbed notebook (`wxAuiNotebook`).
///
/// Used to host a stack of `Panel` children inside an AUI pane. The
/// notebook wraps the same `SysTabControl32` control as
/// [`crate::Tab`], but exposes a friendlier API designed for
/// AUI-style usage (page add / remove, page text change, selection
/// events).
#[cfg(target_os = "windows")]
pub struct AuiNotebook {
    hwnd: HWND,
    pages: Vec<AuiNotebookPage>,
    selection: usize,
    /// Control ID used for WM_NOTIFY dispatch via the parent frame.
    id: u16,
    /// Riferimento alla lista icone: mantiene viva la `ImageList`
    /// nativa per tutta la vita del notebook (senza, l'handle
    /// memorizzato dal controllo diventa dangling quando il creatore
    /// fa cadere la lista a fine scope → icone di tab sparite).
    /// `RefCell` perché `set_image_list` riceve `&self`.
    #[allow(dead_code)]
    image_list: std::cell::RefCell<Option<crate::ImageList>>,
}

#[cfg(target_os = "windows")]
struct AuiNotebookPage {
    title: String,
    child_hwnd: isize,
    image_index: i32,
}

#[cfg(target_os = "windows")]
impl std::fmt::Debug for AuiNotebook {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AuiNotebook")
            .field("page_count", &self.pages.len())
            .field("selection", &self.selection)
            .finish()
    }
}

#[cfg(target_os = "windows")]
mod notebook_consts {
    pub const TCM_FIRST: u32 = 0x1300;
    pub const TCM_ADJUSTRECT: u32 = TCM_FIRST + 40;
    pub const TCM_DELETEALLITEMS: u32 = TCM_FIRST + 9;
    pub const TCM_DELETEITEM: u32 = TCM_FIRST + 8;
    pub const TCM_GETCURSEL: u32 = TCM_FIRST + 11;
    pub const TCM_GETITEMCOUNT: u32 = TCM_FIRST + 4;
    pub const TCM_INSERTITEMW: u32 = TCM_FIRST + 62;
    /// `TCM_SETITEMW` (+61): con +5 arrivavamo a `TCM_GETITEMA` e
    /// `set_page_text`/`set_page_image` non modificavano mai la tab.
    pub const TCM_SETITEMW: u32 = TCM_FIRST + 61;
    pub const TCM_SETCURSEL: u32 = TCM_FIRST + 12;
    pub const TCM_SETIMAGELIST: u32 = TCM_FIRST + 3;
    pub const TCIF_TEXT: u32 = 0x0001;
    pub const TCIF_IMAGE: u32 = 0x0002;

    /// Layout **identico** a `commctrl.h`: i due campi riservati sono
    /// `UINT` (4 byte), non `usize`. Con `usize` su x64 `psz_text`
    /// veniva a offset 24 invece che 16, il controllo leggeva un
    /// testo NULL e le tab apparivano senza etichetta né icona.
    #[repr(C)]
    #[derive(Debug, Clone, Copy)]
    #[allow(clippy::upper_case_acronyms)] // mirrors the Win32 SDK struct name
    pub struct TCITEMHEADERW {
        pub mask: u32,
        pub lp_reserved1: u32,
        pub lp_reserved2: u32,
        pub psz_text: *mut u16,
        pub cch_text_max: i32,
        pub i_image: i32,
    }
}

/// Dispone le pagine sotto la tab strip usando il rettangolo di
/// display calcolato dal controllo (`TCM_ADJUSTRECT`) e mostra solo
/// la pagina `sel`, nascondendo le altre.
#[cfg(target_os = "windows")]
fn layout_notebook_pages(hwnd: HWND, children: &[isize], sel: usize) {
    use windows_sys::Win32::UI::WindowsAndMessaging::SendMessageW;
    // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
    unsafe {
        let mut r: RECT = std::mem::zeroed();
        GetClientRect(hwnd, &mut r);
        SendMessageW(
            hwnd,
            notebook_consts::TCM_ADJUSTRECT,
            0,
            (&mut r as *mut RECT) as isize,
        );
        let x = r.left;
        let y = r.top;
        let w = (r.right - r.left).max(1);
        let h = (r.bottom - r.top).max(1);
        for (idx, child) in children.iter().enumerate() {
            if *child != 0 {
                MoveWindow(*child as HWND, x, y, w, h, 1);
                let show = if idx == sel { SW_SHOW } else { SW_HIDE };
                ShowWindow(*child as HWND, show);
            }
        }
    }
}

/// `WndProc` originale dell'AuiNotebook subclassato.
#[cfg(target_os = "windows")]
type AuiNotebookOrigProc = unsafe extern "system" fn(HWND, u32, WPARAM, LPARAM) -> LRESULT;

#[cfg(target_os = "windows")]
thread_local! {
    /// `WndProc` originale salvato per ogni notebook subclassato.
    static AUI_NOTEBOOK_ORIG_PROCS: std::cell::RefCell<
        std::collections::HashMap<isize, AuiNotebookOrigProc>,
    > = std::cell::RefCell::new(std::collections::HashMap::new());
}

/// Subclassa l'hwnd del notebook per far risalire le notifiche dei
/// controlli figli. `add_page` fa `SetParent` delle pagine sul tab
/// control, quindi un `WM_COMMAND` generato da un `Button` dentro la
/// pagina arriva al wndproc standard di `SysTabControl32` che lo
/// assorbe (`DefWindowProcW` ritorna 0): nessun `on_click` registrato
/// sul frame scattava mai. Ora questi messaggi vengono inoltrati al
/// parent del notebook, che è sempre un `RuWxPanel` (il forwarding
/// porta infine tutto al dispatcher del frame).
#[cfg(target_os = "windows")]
fn install_notebook_forwarding(hwnd: HWND) {
    if hwnd.is_null() {
        return;
    }
    // SAFETY: `hwnd` è il controllo vivo creato da `CreateWindowExW`
    // in `AuiNotebook::new`; salviamo il wndproc di sistema e ne
    // installiamo uno nostro.
    unsafe {
        let key = hwnd as isize;
        let original = GetWindowLongPtrW(hwnd, GWLP_WNDPROC) as usize;
        let original_proc: AuiNotebookOrigProc = std::mem::transmute(original);
        AUI_NOTEBOOK_ORIG_PROCS.with(|m| m.borrow_mut().insert(key, original_proc));
        SetWindowLongPtrW(
            hwnd,
            GWLP_WNDPROC,
            aui_notebook_wnd_proc as *const () as usize as isize,
        );
    }
}

/// `WndProc` del notebook subclassato: inoltra `WM_COMMAND`,
/// `WM_NOTIFY` e le notifiche di scroll al parent (le invia il
/// dispatcher del frame), tutto il resto continua al wndproc di
/// sistema del tab control.
#[cfg(target_os = "windows")]
unsafe extern "system" fn aui_notebook_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_COMMAND | WM_NOTIFY | WM_HSCROLL | WM_VSCROLL => {
            let parent = GetParent(hwnd);
            if !parent.is_null() {
                SendMessageW(parent, msg, wparam, lparam)
            } else {
                0
            }
        }
        _ => {
            let orig = AUI_NOTEBOOK_ORIG_PROCS.with(|m| m.borrow().get(&(hwnd as isize)).copied());
            match orig {
                Some(proc) => proc(hwnd, msg, wparam, lparam),
                None => DefWindowProcW(hwnd, msg, wparam, lparam),
            }
        }
    }
}

#[cfg(target_os = "windows")]
impl AuiNotebook {
    /// Create a notebook child of `parent` (typically a `Panel`).
    /// The notebook HWND is positioned at `(x, y)` with size
    /// `width × height` in the parent's client area.
    pub fn new(parent: &dyn Widget, x: i32, y: i32, width: i32, height: i32) -> Self {
        use windows_sys::Win32::UI::WindowsAndMessaging::CreateWindowExW;
        let id = crate::platform::next_control_id();
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        let hwnd = unsafe {
            CreateWindowExW(
                0,
                to_wide("SysTabControl32").as_ptr(),
                std::ptr::null(),
                WS_CHILD | WS_VISIBLE,
                x,
                y,
                width.max(1),
                height.max(1),
                parent.native_handle() as HWND,
                id as usize as HMENU,
                GetModuleHandleW(std::ptr::null()),
                std::ptr::null_mut(),
            )
        };
        // Le pagine (e i controlli che contengono) diventano figlie
        // del tab control via `SetParent`: senza forwarding i loro
        // `WM_COMMAND` verrebbero assorbiti dal wndproc di sistema.
        install_notebook_forwarding(hwnd);
        Self {
            hwnd,
            pages: Vec::new(),
            selection: 0,
            id,
            image_list: std::cell::RefCell::new(None),
        }
    }

    /// Add a page. The page is a child HWND that will be
    /// re-parented to the notebook and laid out under the tab
    /// strip. Returns the zero-based page index.
    pub fn add_page(&mut self, title: &str, child: &dyn Widget) -> usize {
        use notebook_consts::*;
        use windows_sys::Win32::UI::WindowsAndMessaging::SetParent;
        let index = self.pages.len();
        self.pages.push(AuiNotebookPage {
            title: title.to_string(),
            child_hwnd: child.native_handle(),
            image_index: -1,
        });
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            use windows_sys::Win32::UI::WindowsAndMessaging::SendMessageW;
            let mut buf = to_wide(title);
            let mut item = TCITEMHEADERW {
                // Solo TCIF_TEXT|TCIF_IMAGE: `TCITEMHEADERW` non ha il
                // campo `lParam`, con TCIF_PARAM il controllo avrebbe
                // letto 8 byte oltre la struct.
                mask: TCIF_TEXT | TCIF_IMAGE,
                lp_reserved1: 0,
                lp_reserved2: 0,
                psz_text: buf.as_mut_ptr(),
                cch_text_max: buf.len() as i32,
                i_image: -1,
            };
            SendMessageW(
                self.hwnd,
                TCM_INSERTITEMW,
                index,
                (&mut item as *mut TCITEMHEADERW) as isize,
            );
            SetParent(child.native_handle() as HWND, self.hwnd);
        }
        // Nasconde subito le pagine non attive e posiziona i figli
        // sotto la tab strip (prima tutte le pagine restavano visibili
        // e sovrapposte).
        self.relayout();
        index
    }

    /// Remove the page at `index`. The child HWND is re-parented
    /// to the desktop and hidden.
    pub fn remove_page(&mut self, index: usize) -> bool {
        if index >= self.pages.len() {
            return false;
        }
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            use windows_sys::Win32::UI::WindowsAndMessaging::SendMessageW;
            SendMessageW(self.hwnd, notebook_consts::TCM_DELETEITEM, index, 0);
        }
        let page = self.pages.remove(index);
        #[cfg(target_os = "windows")]
        if page.child_hwnd != 0 {
            // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
            unsafe {
                SetParent(page.child_hwnd as HWND, std::ptr::null_mut());
                ShowWindow(page.child_hwnd as HWND, SW_HIDE);
            }
        }
        if self.selection >= self.pages.len() && !self.pages.is_empty() {
            self.selection = self.pages.len() - 1;
        }
        true
    }

    /// Programmatically set the selected page.
    pub fn set_selection(&mut self, index: usize) {
        if index < self.pages.len() {
            self.selection = index;
            // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
            unsafe {
                use windows_sys::Win32::UI::WindowsAndMessaging::SendMessageW;
                SendMessageW(self.hwnd, notebook_consts::TCM_SETCURSEL, index, 0);
            }
            // Mostra la pagina richiesta e nasconde le altre.
            self.relayout();
        }
    }

    /// Currently selected page (0-based).
    ///
    /// Preferisce la selezione nativa del controllo, così resta
    /// corretta anche dopo un clic dell'utente su una tab.
    pub fn selection(&self) -> usize {
        // SAFETY: TCM_GETCURSEL on a live tab control HWND.
        unsafe {
            use windows_sys::Win32::UI::WindowsAndMessaging::SendMessageW;
            let sel = SendMessageW(self.hwnd, notebook_consts::TCM_GETCURSEL, 0, 0);
            if sel >= 0 {
                return sel as usize;
            }
        }
        self.selection
    }

    /// Number of pages.
    pub fn page_count(&self) -> usize {
        self.pages.len()
    }

    /// Add a page using a native handle directly.
    ///
    /// `child_handle` is a `NativeHandle` (i.e. an `HWND` cast to `isize`)
    /// so callers can pass `panel.hwnd()` directly without a manual cast.
    pub fn add_page_handle(&mut self, title: &str, child_handle: crate::platform::NativeHandle) {
        use notebook_consts::*;
        use windows_sys::Win32::UI::WindowsAndMessaging::{SendMessageW, SetParent};
        let index = self.pages.len();
        self.pages.push(AuiNotebookPage {
            title: title.to_string(),
            child_hwnd: child_handle,
            image_index: -1,
        });
        let mut buf = to_wide(title);
        let mut item = TCITEMHEADERW {
            // Niente TCIF_PARAM: `TCITEMHEADERW` termina con `i_image`.
            mask: TCIF_TEXT | TCIF_IMAGE,
            lp_reserved1: 0,
            lp_reserved2: 0,
            psz_text: buf.as_mut_ptr(),
            cch_text_max: buf.len() as i32,
            i_image: -1,
        };
        unsafe {
            SendMessageW(
                self.hwnd,
                TCM_INSERTITEMW,
                index,
                (&mut item as *mut TCITEMHEADERW) as isize,
            );
            if !child_handle.is_null() {
                SetParent(child_handle as HWND, self.hwnd);
            }
        }
        self.relayout();
    }

    /// Register a callback for tab selection change events.
    ///
    /// Oltre a invocare il callback, la closure ridispose le pagine e
    /// mostra solo quella attiva: senza questo, un clic su una tab
    /// cambiava la selezione nativa ma le pagine restavano tutte
    /// visibili e sovrapposte.
    pub fn on_selection_change<F: FnMut(usize) + 'static>(
        &mut self,
        parent: &Frame,
        mut callback: F,
    ) {
        let hwnd = self.hwnd;
        // Copia degli handle delle pagine: la closure è `'static` e
        // deve poter mostrare/nascondere le pagine al volo.
        let children: Vec<isize> = self.pages.iter().map(|p| p.child_hwnd).collect();
        parent.register_notify_handler(
            self.id,
            Box::new(move |_code| {
                // SAFETY: TCM_GETCURSEL on a live tab control HWND.
                unsafe {
                    use windows_sys::Win32::UI::WindowsAndMessaging::SendMessageW;
                    let sel = SendMessageW(hwnd, notebook_consts::TCM_GETCURSEL, 0, 0);
                    if sel >= 0 {
                        let sel = sel as usize;
                        layout_notebook_pages(hwnd, &children, sel);
                        callback(sel);
                    }
                }
            }),
        );
    }

    /// Native HWND of the tab control.
    pub fn hwnd(&self) -> HWND {
        self.hwnd
    }

    /// Re-layout the page children inside the tab control (call
    /// after a resize).
    pub fn relayout(&self) {
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            use windows_sys::Win32::UI::WindowsAndMessaging::SendMessageW;
            // La selezione affidabile è quella nativa del controllo:
            // si aggiunge anche quando l'utente clicca una tab.
            let sel = SendMessageW(self.hwnd, notebook_consts::TCM_GETCURSEL, 0, 0);
            let sel = if sel >= 0 {
                sel as usize
            } else {
                self.selection
            };
            let children: Vec<isize> = self.pages.iter().map(|p| p.child_hwnd).collect();
            layout_notebook_pages(self.hwnd, &children, sel);
        }
    }

    /// Resize the tab control HWND. Useful when the notebook is
    /// hosted inside an AUI pane and the parent panel was resized
    /// by the manager.
    #[cfg(target_os = "windows")]
    pub fn set_size(&self, w: i32, h: i32) {
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            MoveWindow(self.hwnd, 0, 0, w.max(1), h.max(1), 1);
        }
    }

    #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
    pub fn set_size(&self, _w: i32, _h: i32) {}

    /// Attach an image list used by [`Self::set_page_image`].
    ///
    /// La lista viene **conservata** dal notebook: il clone condivide
    /// lo stesso `HIMAGELIST` (`Arc`), così l'handle resta valido anche
    /// se il chiamante fa cadere la propria `ImageList`.
    pub fn set_image_list(&self, image_list: &crate::ImageList) {
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            use windows_sys::Win32::UI::WindowsAndMessaging::SendMessageW;
            SendMessageW(
                self.hwnd,
                notebook_consts::TCM_SETIMAGELIST,
                0,
                image_list.handle(),
            );
        }
        *self.image_list.borrow_mut() = Some(image_list.clone());
    }

    /// Change the label shown in the tab strip for `index`.
    pub fn set_page_text(&mut self, index: usize, title: &str) -> bool {
        if index >= self.pages.len() {
            return false;
        }
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            use windows_sys::Win32::UI::WindowsAndMessaging::SendMessageW;
            let mut buf = to_wide(title);
            let mut item = notebook_consts::TCITEMHEADERW {
                mask: notebook_consts::TCIF_TEXT,
                lp_reserved1: 0,
                lp_reserved2: 0,
                psz_text: buf.as_mut_ptr(),
                cch_text_max: buf.len() as i32,
                i_image: self.pages[index].image_index,
            };
            let ok = SendMessageW(
                self.hwnd,
                notebook_consts::TCM_SETITEMW,
                index,
                (&mut item as *mut notebook_consts::TCITEMHEADERW) as isize,
            ) != 0;
            if ok {
                self.pages[index].title = title.to_string();
            }
            ok
        }
    }

    /// Set the icon index for a tab page (`-1` removes the icon).
    pub fn set_page_image(&self, index: usize, image_index: i32) -> bool {
        if index >= self.pages.len() {
            return false;
        }
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            use windows_sys::Win32::UI::WindowsAndMessaging::SendMessageW;
            let mut item = notebook_consts::TCITEMHEADERW {
                mask: notebook_consts::TCIF_IMAGE,
                lp_reserved1: 0,
                lp_reserved2: 0,
                psz_text: std::ptr::null_mut(),
                cch_text_max: 0,
                i_image: image_index,
            };
            let ok = SendMessageW(
                self.hwnd,
                notebook_consts::TCM_SETITEMW,
                index,
                (&mut item as *mut notebook_consts::TCITEMHEADERW) as isize,
            ) != 0;
            if ok {
                // Cannot mutate through &self — image_index is cosmetic on Win32.
            }
            ok
        }
    }
}

// ── AuiNotebook (macOS) ──────────────────────────────────────────

#[cfg(target_os = "macos")]
pub struct AuiNotebook {
    tab_handle: crate::platform::NativeHandle,
    parent_handle: crate::platform::NativeHandle,
    rect: Rect,
    pages: Vec<(String, crate::platform::NativeHandle)>,
    selection: usize,
    /// Mantiene viva la `ImageList` nativa (vedi la variante Win32).
    #[allow(dead_code)]
    image_list: std::cell::RefCell<Option<crate::ImageList>>,
}

#[cfg(target_os = "macos")]
impl std::fmt::Debug for AuiNotebook {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AuiNotebook")
            .field("page_count", &self.pages.len())
            .field("selection", &self.selection)
            .finish()
    }
}

#[cfg(target_os = "macos")]
impl AuiNotebook {
    pub fn new(parent: &dyn Widget, x: i32, y: i32, width: i32, height: i32) -> Self {
        let parent_handle = parent.native_handle();
        let rect = Rect::new(x, y, width.max(1) as u32, height.max(1) as u32);
        let tab_handle = crate::platform::appkit::create_tab(parent_handle, rect);
        Self {
            tab_handle,
            parent_handle,
            rect,
            pages: Vec::new(),
            selection: 0,
            image_list: std::cell::RefCell::new(None),
        }
    }

    pub fn add_page(&mut self, title: &str, child: &dyn Widget) -> usize {
        let child_handle = child.native_handle();
        self.add_page_handle(title, child_handle)
    }

    /// Add a page from an already-resolved native handle (avoids holding
    /// a panel `RefCell` borrow across tab layout).
    pub fn add_page_handle(
        &mut self,
        title: &str,
        child_handle: crate::platform::NativeHandle,
    ) -> usize {
        let index = crate::platform::appkit::tab_add_page(self.tab_handle, title, child_handle);
        self.pages.push((title.to_string(), child_handle));
        if index >= 0 {
            self.selection = index as usize;
            crate::platform::appkit::tab_select_index(self.tab_handle, self.selection);
        }
        self.pages.len().saturating_sub(1)
    }

    pub fn remove_page(&mut self, index: usize) -> bool {
        if index >= self.pages.len() {
            return false;
        }
        crate::platform::appkit::tab_remove_page(self.tab_handle, index);
        self.pages.remove(index);
        if self.selection >= self.pages.len() && !self.pages.is_empty() {
            self.selection = self.pages.len() - 1;
        }
        true
    }

    pub fn set_selection(&mut self, index: usize) {
        if index < self.pages.len() {
            self.selection = index;
            crate::platform::appkit::tab_set_selection(self.tab_handle, index);
            if let Some((_, child)) = self.pages.get(index) {
                crate::platform::appkit::tab_layout_page(self.tab_handle, *child);
            }
        }
    }

    pub fn selection(&self) -> usize {
        self.selection
    }

    pub fn page_count(&self) -> usize {
        self.pages.len()
    }

    pub fn relayout(&self) {
        for (_, child) in &self.pages {
            crate::platform::appkit::tab_layout_page(self.tab_handle, *child);
        }
    }

    pub fn set_size(&self, w: i32, h: i32) {
        let mut rect = self.rect;
        rect.width = w.max(1) as u32;
        rect.height = h.max(1) as u32;
        crate::platform::appkit::set_view_frame(self.parent_handle, self.tab_handle, rect);
        self.relayout();
    }

    pub fn set_image_list(&self, image_list: &crate::ImageList) {
        crate::platform::appkit::tab_set_image_list(self.tab_handle, image_list.handle());
        // Mantieni viva la lista nativa per tutta la vita del notebook.
        *self.image_list.borrow_mut() = Some(image_list.clone());
    }

    pub fn set_page_text(&mut self, index: usize, title: &str) -> bool {
        if index >= self.pages.len() {
            return false;
        }
        if crate::platform::appkit::tab_set_page_text(self.tab_handle, index, title) {
            self.pages[index].0 = title.to_string();
            true
        } else {
            false
        }
    }

    pub fn set_page_image(&self, index: usize, image_index: i32) -> bool {
        if index >= self.pages.len() {
            return false;
        }
        crate::platform::appkit::tab_set_page_image(self.tab_handle, index, image_index);
        true
    }

    pub fn on_selection_change<F: FnMut(usize) + 'static>(
        &mut self,
        _frame: &crate::Frame,
        mut callback: F,
    ) {
        let tab_handle = self.tab_handle;
        crate::platform::appkit::set_tab_selection_handler(
            tab_handle,
            Box::new(move |index| {
                callback(index);
            }),
        );
    }

    pub fn set_tab_chrome(&self, chrome: crate::platform::appkit::TabViewChrome) {
        crate::platform::appkit::tab_set_view_chrome(self.tab_handle, chrome);
        crate::platform::appkit::tab_relayout_all_pages(self.tab_handle);
    }

    pub fn set_tab_control_size(&self, size: crate::platform::appkit::TabControlSize) {
        crate::platform::appkit::tab_set_control_size(self.tab_handle, size);
        crate::platform::appkit::tab_relayout_all_pages(self.tab_handle);
    }

    pub fn set_tab_draws_background(&self, draws: bool) {
        crate::platform::appkit::tab_set_draws_background(self.tab_handle, draws);
    }
}

// ── AuiNotebook (Linux) ─────────────────────────────────────────

#[cfg(target_os = "linux")]
pub struct AuiNotebook {
    tab_handle: crate::platform::NativeHandle,
    parent_handle: crate::platform::NativeHandle,
    rect: Rect,
    pages: Vec<(String, crate::platform::NativeHandle)>,
    selection: usize,
    /// Mantiene viva la `ImageList` nativa (come nella variante macOS).
    #[allow(dead_code)]
    image_list: std::cell::RefCell<Option<crate::ImageList>>,
}

#[cfg(target_os = "linux")]
impl std::fmt::Debug for AuiNotebook {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AuiNotebook")
            .field("page_count", &self.pages.len())
            .field("selection", &self.selection)
            .finish()
    }
}

#[cfg(target_os = "linux")]
impl AuiNotebook {
    pub fn new(parent: &dyn Widget, x: i32, y: i32, width: i32, height: i32) -> Self {
        let parent_handle = parent.native_handle();
        let rect = Rect::new(x, y, width.max(1) as u32, height.max(1) as u32);
        let tab_handle = crate::platform::gtk::create_tab(parent_handle, rect);
        Self {
            tab_handle,
            parent_handle,
            rect,
            pages: Vec::new(),
            selection: 0,
            image_list: std::cell::RefCell::new(None),
        }
    }

    pub fn add_page(&mut self, title: &str, child: &dyn Widget) -> usize {
        let child_handle = child.native_handle();
        self.add_page_handle(title, child_handle)
    }

    /// Add a page from an already-resolved native handle (avoids holding
    /// a panel `RefCell` borrow across tab layout).
    pub fn add_page_handle(
        &mut self,
        title: &str,
        child_handle: crate::platform::NativeHandle,
    ) -> usize {
        let index = crate::platform::gtk::tab_add_page(self.tab_handle, title, child_handle);
        self.pages.push((title.to_string(), child_handle));
        if index >= 0 {
            self.selection = index as usize;
            crate::platform::gtk::tab_select_index(self.tab_handle, self.selection);
        }
        self.pages.len().saturating_sub(1)
    }

    pub fn remove_page(&mut self, index: usize) -> bool {
        if index >= self.pages.len() {
            return false;
        }
        crate::platform::gtk::tab_remove_page(self.tab_handle, index);
        self.pages.remove(index);
        if self.selection >= self.pages.len() && !self.pages.is_empty() {
            self.selection = self.pages.len() - 1;
        }
        true
    }

    pub fn set_selection(&mut self, index: usize) {
        if index < self.pages.len() {
            self.selection = index;
            crate::platform::gtk::tab_set_selection(self.tab_handle, index);
            if let Some((_, child)) = self.pages.get(index) {
                crate::platform::gtk::tab_layout_page(self.tab_handle, *child);
            }
        }
    }

    pub fn selection(&self) -> usize {
        self.selection
    }

    pub fn page_count(&self) -> usize {
        self.pages.len()
    }

    pub fn relayout(&self) {
        for (_, child) in &self.pages {
            crate::platform::gtk::tab_layout_page(self.tab_handle, *child);
        }
    }

    pub fn set_size(&self, w: i32, h: i32) {
        let mut rect = self.rect;
        rect.width = w.max(1) as u32;
        rect.height = h.max(1) as u32;
        crate::platform::gtk::set_view_frame(self.parent_handle, self.tab_handle, rect);
        self.relayout();
    }

    pub fn set_image_list(&self, image_list: &crate::ImageList) {
        crate::platform::gtk::tab_set_image_list(self.tab_handle, image_list.handle());
        // Mantieni viva la lista nativa per tutta la vita del notebook.
        *self.image_list.borrow_mut() = Some(image_list.clone());
    }

    pub fn set_page_text(&mut self, index: usize, title: &str) -> bool {
        if index >= self.pages.len() {
            return false;
        }
        if crate::platform::gtk::tab_set_page_text(self.tab_handle, index, title) {
            self.pages[index].0 = title.to_string();
            true
        } else {
            false
        }
    }

    pub fn set_page_image(&self, index: usize, _image_index: i32) -> bool {
        if index >= self.pages.len() {
            return false;
        }
        // `ImageList` è ancora uno stub su Linux: la chiamata è un no-op.
        true
    }

    pub fn on_selection_change<F: FnMut(usize) + 'static>(
        &mut self,
        _frame: &crate::Frame,
        mut callback: F,
    ) {
        let tab_handle = self.tab_handle;
        crate::platform::gtk::set_tab_selection_handler(
            tab_handle,
            Box::new(move |index| {
                callback(index);
            }),
        );
    }
}

// Stub implementation for other platforms so the API is still available.
#[cfg(all(
    not(target_os = "windows"),
    not(target_os = "macos"),
    not(target_os = "linux")
))]
#[derive(Debug, Default)]
pub struct AuiNotebook {
    pages: Vec<String>,
    selection: usize,
}

#[cfg(all(
    not(target_os = "windows"),
    not(target_os = "macos"),
    not(target_os = "linux")
))]
impl AuiNotebook {
    pub fn new(_parent: &dyn Widget, _x: i32, _y: i32, _width: i32, _height: i32) -> Self {
        Self::default()
    }
    pub fn add_page(&mut self, title: &str, _child: &dyn Widget) -> usize {
        self.pages.push(title.to_string());
        self.pages.len() - 1
    }
    pub fn remove_page(&mut self, index: usize) -> bool {
        if index < self.pages.len() {
            self.pages.remove(index);
            true
        } else {
            false
        }
    }
    pub fn set_selection(&mut self, index: usize) {
        if index < self.pages.len() {
            self.selection = index;
        }
    }
    pub fn selection(&self) -> usize {
        self.selection
    }
    pub fn page_count(&self) -> usize {
        self.pages.len()
    }
    pub fn relayout(&self) {}
    pub fn set_size(&self, _w: i32, _h: i32) {}

    /// Versione stub: accetta la `ImageList` senza applicarla.
    pub fn set_image_list(&self, _image_list: &crate::ImageList) {}

    /// Versione stub da `add_page`, con handle nativo già risolto.
    pub fn add_page_handle(
        &mut self,
        title: &str,
        _child_handle: crate::platform::NativeHandle,
    ) -> usize {
        self.pages.push(title.to_string());
        self.pages.len() - 1
    }

    pub fn set_page_text(&mut self, index: usize, title: &str) -> bool {
        if index >= self.pages.len() {
            return false;
        }
        self.pages[index] = title.to_string();
        true
    }

    pub fn set_page_image(&self, index: usize, _image_index: i32) -> bool {
        index < self.pages.len()
    }

    pub fn on_selection_change<F: FnMut(usize) + 'static>(
        &mut self,
        _frame: &crate::Frame,
        _callback: F,
    ) {
    }
}

// ── AuiManager ───────────────────────────────────────────────────

/// Main docking manager (`wxAuiManager`).
///
/// The manager owns a list of [`AuiPaneInfo`] entries and a
/// [`AuiDockArt`]. Call [`AuiManager::add_pane`] to register a
/// pane, [`AuiManager::update`] to apply the current layout, and
/// [`AuiManager::show_pane`] / [`AuiManager::hide_pane`] to
/// toggle individual panes.
///
/// The manager does **not** own the pane HWNDs; it only repositions
/// them. The user is responsible for keeping the HWNDs alive (e.g.
/// by storing the corresponding `Widget` in scope).
pub struct AuiManager {
    panes: Vec<AuiPaneInfo>,
    art: AuiDockArt,
    /// The first pane created with [`AuiPaneInfo::center`] is
    /// treated as the "center" pane. The center pane always fills
    /// whatever space is left after docked side panes are placed.
    center_pane_index: Option<usize>,
}

impl Default for AuiManager {
    fn default() -> Self {
        Self::new()
    }
}

impl AuiManager {
    pub fn new() -> Self {
        Self {
            panes: Vec::new(),
            art: AuiDockArt::new(),
            center_pane_index: None,
        }
    }

    /// Access the dock art / theme for tweaking.
    pub fn dock_art(&self) -> &AuiDockArt {
        &self.art
    }
    /// Mutable access to the dock art / theme.
    pub fn dock_art_mut(&mut self) -> &mut AuiDockArt {
        &mut self.art
    }

    /// Register a pane with the manager. The pane is **not** shown
    /// until [`AuiManager::update`] is called. If the pane is a
    /// center pane (built with [`AuiPaneInfo::center`]) it replaces
    /// any previously-registered center pane.
    pub fn add_pane(&mut self, mut info: AuiPaneInfo) {
        if info.is_center() {
            // Replace any prior center pane.
            self.panes.retain(|p| !p.is_center());
            let new_idx = self.panes.len();
            self.center_pane_index = Some(new_idx);
            info.floating = false;
            self.panes.push(info);
        } else {
            self.panes.push(info);
        }
    }

    /// Get a read-only reference to the pane with the given name.
    pub fn get_pane(&self, name: &str) -> Option<&AuiPaneInfo> {
        self.panes.iter().find(|p| p.name == name)
    }

    /// Get a mutable reference to the pane with the given name.
    pub fn get_pane_mut(&mut self, name: &str) -> Option<&mut AuiPaneInfo> {
        self.panes.iter_mut().find(|p| p.name == name)
    }

    /// Show the pane with the given name and re-layout the frame.
    pub fn show_pane(&mut self, name: &str) -> bool {
        if let Some(p) = self.get_pane_mut(name) {
            p.shown = true;
            true
        } else {
            false
        }
    }

    /// Hide the pane with the given name and re-layout the frame.
    pub fn hide_pane(&mut self, name: &str) -> bool {
        if let Some(p) = self.get_pane_mut(name) {
            p.shown = false;
            true
        } else {
            false
        }
    }

    /// Detach a docked pane into a floating top-level window.
    pub fn float_pane(&mut self, frame: &Frame, name: &str, screen_x: i32, screen_y: i32) -> bool {
        let Some(info) = self.get_pane(name).map(|p| {
            (
                p.floatable,
                p.floating,
                p.shown,
                p.pinned,
                p.rect,
                p.caption.clone(),
            )
        }) else {
            return false;
        };
        let (floatable, floating, shown, pinned, rect, caption) = info;
        if !floatable || floating || !shown || pinned {
            return false;
        }
        let w = rect.width.max(120);
        let h = rect.height.max(80);
        let float_rect = Rect::new(screen_x.max(0), screen_y.max(0), w, h);
        if let Some(p) = self.get_pane_mut(name) {
            p.floating = true;
        }
        let frame_handle = frame.hwnd();
        #[cfg(target_os = "macos")]
        crate::platform::appkit::float_aui_pane_host(frame_handle, name, &float_rect, &caption);
        #[cfg(target_os = "windows")]
        crate::platform::win32_aui::float_aui_pane_host(frame_handle, name, &float_rect, &caption);
        #[cfg(target_os = "linux")]
        crate::platform::gtk::float_aui_pane_host(frame_handle, name, &float_rect, &caption);
        self.update(frame);
        true
    }

    /// Collapse or expand an auto-hide pane. No-op if [`AuiPaneInfo::auto_hide`]
    /// is `false`.
    pub fn toggle_auto_hide_pane(&mut self, frame: &Frame, name: &str) -> bool {
        let Some(p) = self.get_pane_mut(name) else {
            return false;
        };
        if !p.auto_hide || p.floating {
            return false;
        }
        if p.auto_hidden {
            p.best_size = p.saved_best_size;
            p.auto_hidden = false;
        } else {
            p.saved_best_size = p.best_size;
            p.auto_hidden = true;
        }
        self.update(frame);
        true
    }

    /// Resize a docked pane after the user dragged an AUI sash.
    pub fn apply_sash_resize(
        &mut self,
        frame: &Frame,
        pane_name: &str,
        delta: i32,
        vertical: bool,
    ) -> bool {
        let Some(p) = self.get_pane_mut(pane_name) else {
            return false;
        };
        if p.floating || p.auto_hidden {
            return false;
        }
        match p.dock_side {
            AuiDockSide::Left if vertical => {
                let mut nw = p.best_size.0 + delta;
                nw = nw.max(p.min_size.0);
                if p.max_size.0 > 0 {
                    nw = nw.min(p.max_size.0);
                }
                p.best_size.0 = nw;
            }
            AuiDockSide::Right if vertical => {
                let mut nw = p.best_size.0 - delta;
                nw = nw.max(p.min_size.0);
                if p.max_size.0 > 0 {
                    nw = nw.min(p.max_size.0);
                }
                p.best_size.0 = nw;
            }
            AuiDockSide::Top if !vertical => {
                let mut nh = p.best_size.1 + delta;
                nh = nh.max(p.min_size.1);
                if p.max_size.1 > 0 {
                    nh = nh.min(p.max_size.1);
                }
                p.best_size.1 = nh;
            }
            AuiDockSide::Bottom if !vertical => {
                let mut nh = p.best_size.1 - delta;
                nh = nh.max(p.min_size.1);
                if p.max_size.1 > 0 {
                    nh = nh.min(p.max_size.1);
                }
                p.best_size.1 = nh;
            }
            _ => return false,
        }
        self.update(frame);
        true
    }

    /// Re-dock a pane that was previously detached with [`Self::float_pane`].
    pub fn dock_pane(&mut self, frame: &Frame, name: &str) -> bool {
        if !self.get_pane(name).is_some_and(|p| p.floating) {
            return false;
        }
        if let Some(p) = self.get_pane_mut(name) {
            p.floating = false;
        }
        let frame_handle = frame.hwnd();
        #[cfg(target_os = "macos")]
        crate::platform::appkit::dock_aui_pane_host(frame_handle, name);
        #[cfg(target_os = "windows")]
        crate::platform::win32_aui::dock_aui_pane_host(frame_handle, name);
        #[cfg(target_os = "linux")]
        crate::platform::gtk::dock_aui_pane_host(frame_handle, name);
        self.update(frame);
        true
    }

    pub(crate) fn handle_pane_button(&mut self, frame: &Frame, name: &str, button: AuiPaneButton) {
        match button {
            AuiPaneButton::Close => {
                self.hide_pane(name);
                self.update(frame);
            }
            AuiPaneButton::Maximize => {
                if let Some(p) = self.get_pane_mut(name) {
                    match p.dock_side {
                        AuiDockSide::Left | AuiDockSide::Right => {
                            let next = p.best_size.0.saturating_mul(2).max(p.min_size.0);
                            p.best_size.0 = if next == p.best_size.0 {
                                p.min_size.0.max(200)
                            } else {
                                next
                            };
                        }
                        _ => {
                            let next = p.best_size.1.saturating_mul(2).max(p.min_size.1);
                            p.best_size.1 = if next == p.best_size.1 {
                                p.min_size.1.max(120)
                            } else {
                                next
                            };
                        }
                    }
                    self.update(frame);
                }
            }
            AuiPaneButton::Pin => {
                if let Some(p) = self.get_pane_mut(name) {
                    p.pinned = !p.pinned;
                    let pinned = p.pinned;
                    let frame_handle = frame.hwnd();
                    #[cfg(target_os = "macos")]
                    crate::platform::appkit::sync_aui_pane_host_pin(frame_handle, name, pinned);
                    #[cfg(target_os = "windows")]
                    crate::platform::win32_aui::sync_aui_pane_host_pin(frame_handle, name, pinned);
                    #[cfg(target_os = "linux")]
                    crate::platform::gtk::sync_aui_pane_host_pin(frame_handle, name, pinned);
                    self.update(frame);
                }
            }
            AuiPaneButton::Minimize | AuiPaneButton::Custom(_) => {}
        }
    }

    fn register_pane_button_handlers(&mut self, frame: &Frame) {
        let mgr_ptr = self as *mut Self;
        let frame_hwnd = frame.hwnd();
        #[cfg(target_os = "macos")]
        let commands = crate::platform::appkit::pane_button_commands_for_frame(frame_hwnd);
        #[cfg(target_os = "windows")]
        let commands = crate::platform::win32_aui::pane_button_commands_for_frame(frame_hwnd);
        #[cfg(target_os = "linux")]
        let commands = crate::platform::gtk::pane_button_commands_for_frame(frame_hwnd);
        #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
        #[allow(clippy::single_match_else, unused_variables)]
        let commands: Vec<(u16, String, AuiPaneButton)> = Vec::new();
        for (cmd_id, pane_name, button) in commands {
            let frame_for_handler = frame.clone();
            let pane_name = pane_name.clone();
            frame.register_command_handler(
                cmd_id,
                Box::new(move || {
                    // SAFETY: `AuiManager` must outlive the frame (typical usage keeps it in
                    // `Rc<RefCell<_>>` beside the frame). The pointer is refreshed on every
                    // `update` call.
                    unsafe {
                        if let Some(mgr) = mgr_ptr.as_mut() {
                            mgr.handle_pane_button(&frame_for_handler, &pane_name, button);
                        }
                    }
                }),
            );
        }
    }

    fn register_pane_action_target(&mut self, frame: &Frame) {
        let mgr_ptr = self as *mut Self;
        let frame_clone = frame.clone();
        #[cfg(target_os = "macos")]
        crate::platform::appkit::register_aui_pane_action_target(
            frame.hwnd(),
            mgr_ptr,
            frame_clone,
        );
        #[cfg(target_os = "windows")]
        crate::platform::win32_aui::register_aui_pane_action_target(
            frame.hwnd(),
            mgr_ptr,
            frame_clone,
        );
        #[cfg(target_os = "linux")]
        crate::platform::gtk::register_aui_pane_action_target(frame.hwnd(), mgr_ptr, frame_clone);
    }

    #[cfg(target_os = "macos")]
    fn platform_position_pane(
        art: &AuiDockArt,
        frame_handle: crate::platform::NativeHandle,
        parent: crate::platform::NativeHandle,
        p: &mut AuiPaneInfo,
        rect: Rect,
        layout_child: &dyn Fn(crate::platform::NativeHandle, crate::platform::NativeHandle, Rect),
    ) {
        if p.caption_visible {
            if let Some(child) = p.window_handle.filter(|&h| h != 0) {
                let host = crate::platform::appkit::ensure_aui_pane_host(
                    frame_handle,
                    parent,
                    &p.name,
                    child,
                    p.dock_side,
                    &p.caption,
                    &p.buttons,
                    art,
                    p.floatable,
                    p.pinned,
                );
                crate::platform::appkit::position_aui_pane_host(
                    parent,
                    host,
                    rect,
                    art.caption_height(),
                );
                crate::platform::appkit::sync_aui_pane_auto_hide(
                    frame_handle,
                    &p.name,
                    p.auto_hide,
                    p.auto_hidden,
                );
                if p.is_toolbar {
                    crate::platform::appkit::toolbar_set_managed_by_aui(child, true);
                    crate::platform::appkit::toolbar_sync_host_layout(child);
                    crate::platform::appkit::toolbar_bring_to_front(child);
                }
            }
        } else if let Some(child) = p.window_handle.filter(|&h| h != 0) {
            layout_child(parent, child, rect);
            if p.is_toolbar {
                crate::platform::appkit::toolbar_set_managed_by_aui(child, true);
                crate::platform::appkit::toolbar_sync_host_layout(child);
                crate::platform::appkit::toolbar_bring_to_front(child);
            }
        }
        p.rect = rect;
    }

    #[cfg(target_os = "windows")]
    fn platform_position_pane(
        art: &AuiDockArt,
        frame_handle: crate::platform::NativeHandle,
        parent: crate::platform::NativeHandle,
        p: &mut AuiPaneInfo,
        rect: Rect,
        layout_child: &dyn Fn(crate::platform::NativeHandle, crate::platform::NativeHandle, Rect),
    ) {
        if p.caption_visible && !p.is_toolbar {
            if let Some(child) = p.window_handle.filter(|&h| h != 0) {
                let host = crate::platform::win32_aui::ensure_aui_pane_host(
                    frame_handle,
                    parent,
                    &p.name,
                    child,
                    p.dock_side,
                    &p.caption,
                    &p.buttons,
                    art,
                    p.floatable,
                    p.pinned,
                );
                crate::platform::win32_aui::position_aui_pane_host(
                    parent,
                    host,
                    rect,
                    art.caption_height(),
                );
            }
        } else if let Some(child) = p.window_handle.filter(|&h| h != 0) {
            layout_child(parent, child, rect);
        }
        p.rect = rect;
    }

    #[cfg(target_os = "linux")]
    fn platform_position_pane(
        art: &AuiDockArt,
        frame_handle: crate::platform::NativeHandle,
        parent: crate::platform::NativeHandle,
        p: &mut AuiPaneInfo,
        rect: Rect,
        layout_child: &dyn Fn(crate::platform::NativeHandle, crate::platform::NativeHandle, Rect),
    ) {
        if p.caption_visible {
            if let Some(child) = p.window_handle.filter(|&h| h != 0) {
                let host = crate::platform::gtk::ensure_aui_pane_host(
                    frame_handle,
                    parent,
                    &p.name,
                    child,
                    p.dock_side,
                    &p.caption,
                    &p.buttons,
                    art,
                    p.floatable,
                    p.pinned,
                );
                crate::platform::gtk::position_aui_pane_host(
                    parent,
                    host,
                    rect,
                    art.caption_height(),
                );
                crate::platform::gtk::sync_aui_pane_auto_hide(
                    frame_handle,
                    &p.name,
                    p.auto_hide,
                    p.auto_hidden,
                );
                if p.is_toolbar {
                    crate::platform::gtk::toolbar_set_managed_by_aui(child, true);
                    crate::platform::gtk::toolbar_sync_host_layout(child);
                    crate::platform::gtk::toolbar_bring_to_front(child);
                }
            }
        } else if let Some(child) = p.window_handle.filter(|&h| h != 0) {
            layout_child(parent, child, rect);
            if p.is_toolbar {
                crate::platform::gtk::toolbar_set_managed_by_aui(child, true);
                crate::platform::gtk::toolbar_sync_host_layout(child);
                crate::platform::gtk::toolbar_bring_to_front(child);
            }
        }
        p.rect = rect;
    }

    /// All registered panes, in insertion order.
    pub fn panes(&self) -> &[AuiPaneInfo] {
        &self.panes
    }

    /// Number of registered panes.
    pub fn pane_count(&self) -> usize {
        self.panes.len()
    }

    /// Position registered pane HWNDs inside `frame`'s client area.
    ///
    /// The layout is computed in two passes:
    ///
    /// 1. **Vertical strips** — top and bottom panes consume a
    ///    horizontal strip whose height is the pane's `best_size`
    ///    (or `min_size` if the strip would otherwise be smaller).
    ///    Multiple panes on the same edge stack in `row` order
    ///    inside their `layer`; higher layers push toward the
    ///    center.
    /// 2. **Horizontal strips** — left and right panes consume a
    ///    vertical strip whose width is the pane's `best_size`.
    ///    Stacking rules mirror the top / bottom case.
    /// 3. **Center** — the center pane (if any) fills whatever
    ///    rectangle is left after the strips have been reserved.
    ///
    /// Sashes are not drawn by the manager; the host application
    /// can draw them on top of the manager's paint pass.
    pub fn update(&mut self, frame: &Frame) {
        #[cfg(target_os = "windows")]
        {
            let hwnd = frame.hwnd();
            if hwnd.is_null() {
                return;
            }
            let mut client = RECT {
                left: 0,
                top: 0,
                right: 0,
                bottom: 0,
            };
            // SAFETY: client rect query on a live frame HWND.
            unsafe {
                GetClientRect(as_hwnd(hwnd), &mut client);
            }
            let cw = (client.right - client.left).max(0);
            let ch = (client.bottom - client.top).max(0);
            // All user-supplied sizes (pane padding, caption height,
            // `best_size` / `min_size` on the panes) are in 96-DPI
            // logical pixels. Convert them to the same unit that
            // `GetClientRect` / `MoveWindow` use so the layout math
            // is unit-consistent. See the long comment on `scale`
            // below for the full rationale.
            let padding_log = self.art.pane_padding() as i32;
            let caption_log = self.art.caption_height() as i32;

            // Query the frame's effective DPI so we can convert the
            // user-supplied logical sizes (`best_size`, `min_size`,
            // the caption strip, the pane padding) to the same unit
            // that `GetClientRect` / `MoveWindow` use. The unit
            // depends on the process's DPI awareness:
            //
            // * PerMonitorV2 aware: `GetClientRect` returns logical
            //   pixels; `MoveWindow` expects logical pixels. No
            //   scaling is needed (the factor is 1.0).
            // * DPI-unaware / System aware: `GetClientRect` returns
            //   physical pixels; `MoveWindow` expects physical
            //   pixels. The user-supplied logical values must be
            //   multiplied by the scale factor before they are used
            //   as the w/h of `MoveWindow` calls.
            //
            // We always convert to physical pixels internally so the
            // layout math is unit-consistent regardless of the
            // process awareness state. The user-facing API stays
            // in 96-DPI (logical) units.
            //
            // The library declares `PerMonitorV2` awareness in its
            // `app.manifest` and `App::new` re-asserts it at
            // runtime. In that mode, `GetClientRect` returns
            // **logical** pixels and `MoveWindow` accepts
            // **logical** pixels, so the conversion factor is 1.0
            // and `scale_log` is the identity. If the manifest
            // ever falls off (e.g. a launcher strips it) the
            // process reverts to DPI-unaware, `GetClientRect` /
            // `MoveWindow` flip to physical pixels, and the
            // per-window `GetDpiForWindow` keeps reporting the
            // correct per-monitor value — so a single
            // `dpi.scale_factor()` lookup produces a
            // self-consistent unit system: `MoveWindow` in
            // physical, `cw`/`ch` (from `GetClientRect`) in
            // physical, and the layout math in physical.
            let _scale = get_dpi_for_window(as_hwnd(hwnd)).scale_factor();
            let _scale_log = |v: i32| -> i32 { v };
            let padding = padding_log;
            let caption = caption_log;
            let art = &self.art;
            let frame_handle = hwnd as crate::platform::NativeHandle;
            let parent = frame_handle;

            let layout_child = |parent: crate::platform::NativeHandle,
                                child: crate::platform::NativeHandle,
                                rect: Rect| {
                let _ = parent;
                // SAFETY: MoveWindow on a registered child HWND.
                unsafe {
                    MoveWindow(
                        child as _,
                        rect.x,
                        rect.y,
                        rect.width as i32,
                        rect.height as i32,
                        1,
                    );
                }
            };

            // ── TOP strip ───────────────────────────────────────────
            // Panes sorted by (layer, row) — outer first.
            let mut top: Vec<&mut AuiPaneInfo> = self
                .panes
                .iter_mut()
                .filter(|p| p.shown && !p.floating && p.dock_side == AuiDockSide::Top)
                .collect();
            top.sort_by_key(|p| (p.layer, p.row));
            let mut top_y = 0i32;
            for p in &mut top {
                let h = docked_pane_height(p, caption);
                let rect = Rect::new(0, top_y, cw as u32, h as u32);
                Self::platform_position_pane(art, frame_handle, parent, p, rect, &layout_child);
                top_y += h + padding;
            }

            // ── BOTTOM strip ────────────────────────────────────────
            let mut bottom: Vec<&mut AuiPaneInfo> = self
                .panes
                .iter_mut()
                .filter(|p| p.shown && !p.floating && p.dock_side == AuiDockSide::Bottom)
                .collect();
            bottom.sort_by_key(|p| (p.layer, p.row));
            let mut bottom_y = ch;
            let mut bottom_pane_h: i32 = 0;
            for p in &mut bottom {
                let h = docked_pane_height(p, caption);
                bottom_y -= h;
                let rect = Rect::new(0, bottom_y, cw as u32, h as u32);
                Self::platform_position_pane(art, frame_handle, parent, p, rect, &layout_child);
                bottom_y -= padding;
                bottom_pane_h += h + padding;
            }
            let center_top = top_y;
            let center_bottom = ch - bottom_pane_h;
            let center_h = (center_bottom - center_top).max(0);

            // ── LEFT strip ──────────────────────────────────────────
            let mut left: Vec<&mut AuiPaneInfo> = self
                .panes
                .iter_mut()
                .filter(|p| p.shown && !p.floating && p.dock_side == AuiDockSide::Left)
                .collect();
            left.sort_by_key(|p| (p.layer, p.row));
            let mut left_x = 0i32;
            for p in &mut left {
                let w = docked_pane_width(p, caption);
                let rect = Rect::new(left_x, center_top, w as u32, center_h as u32);
                Self::platform_position_pane(art, frame_handle, parent, p, rect, &layout_child);
                left_x += w + padding;
            }

            // ── RIGHT strip ─────────────────────────────────────────
            let mut right: Vec<&mut AuiPaneInfo> = self
                .panes
                .iter_mut()
                .filter(|p| p.shown && !p.floating && p.dock_side == AuiDockSide::Right)
                .collect();
            right.sort_by_key(|p| (p.layer, p.row));
            let mut right_x = cw;
            let mut right_pane_w: i32 = 0;
            for p in &mut right {
                let w = docked_pane_width(p, caption);
                right_x -= w;
                let rect = Rect::new(right_x, center_top, w as u32, center_h as u32);
                Self::platform_position_pane(art, frame_handle, parent, p, rect, &layout_child);
                right_x -= padding;
                right_pane_w += w + padding;
            }
            let center_x = left_x;
            let center_w = (cw - left_x - right_pane_w).max(0);

            // ── CENTER ─────────────────────────────────────────────
            if let Some(idx) = self.center_pane_index {
                if let Some(p) = self.panes.get_mut(idx) {
                    if p.shown && !p.floating {
                        let rect =
                            Rect::new(center_x, center_top, center_w as u32, center_h as u32);
                        Self::platform_position_pane(
                            art,
                            frame_handle,
                            parent,
                            p,
                            rect,
                            &layout_child,
                        );
                    }
                }
            }

            // ── Show / hide HWNDs to match `shown` ──────────────────
            // SAFETY: ShowWindow calls.
            unsafe {
                for p in &self.panes {
                    let cmd = if p.shown { SW_SHOW } else { SW_HIDE };
                    if p.caption_visible && !p.is_toolbar {
                        if let Some(host) =
                            crate::platform::win32_aui::aui_pane_host_handle(frame_handle, &p.name)
                        {
                            ShowWindow(host as _, cmd);
                            continue;
                        }
                    }
                    if let Some(child) = p.window_handle.filter(|&h| h != 0) {
                        ShowWindow(child as _, cmd);
                    }
                }
            }

            self.register_pane_button_handlers(frame);
            self.register_pane_action_target(frame);
            Self::sync_sashes_after_layout(self, frame, parent);
        }
        #[cfg(target_os = "macos")]
        {
            let parent = frame.hwnd();
            if parent == 0 {
                return;
            }
            let frame_handle = parent;
            let (cw_u, ch_u) = crate::platform::appkit::frame_client_size(parent);
            let cw = cw_u as i32;
            let ch = ch_u as i32;
            let padding = self.art.pane_padding() as i32;
            let caption = self.art.caption_height() as i32;
            let side_caption =
                crate::platform::appkit::aui_side_caption_width(self.art.caption_height()) as i32;
            let art = &self.art;

            let layout_child = |parent: crate::platform::NativeHandle,
                                child: crate::platform::NativeHandle,
                                rect: Rect| {
                crate::platform::appkit::set_view_frame(parent, child, rect);
            };

            let mut top: Vec<&mut AuiPaneInfo> = self
                .panes
                .iter_mut()
                .filter(|p| p.shown && !p.floating && p.dock_side == AuiDockSide::Top)
                .collect();
            top.sort_by_key(|p| (p.layer, p.row));
            let mut top_y = 0i32;
            for p in &mut top {
                let h = docked_pane_height(p, caption);
                let rect = Rect::new(0, top_y, cw_u, h as u32);
                Self::platform_position_pane(art, frame_handle, parent, p, rect, &layout_child);
                top_y += h + padding;
            }

            let mut bottom: Vec<&mut AuiPaneInfo> = self
                .panes
                .iter_mut()
                .filter(|p| p.shown && !p.floating && p.dock_side == AuiDockSide::Bottom)
                .collect();
            bottom.sort_by_key(|p| (p.layer, p.row));
            let mut bottom_y = ch;
            let mut bottom_pane_h: i32 = 0;
            for p in &mut bottom {
                let h = docked_pane_height(p, caption);
                bottom_y -= h;
                let rect = Rect::new(0, bottom_y, cw_u, h as u32);
                Self::platform_position_pane(art, frame_handle, parent, p, rect, &layout_child);
                bottom_y -= padding;
                bottom_pane_h += h + padding;
            }
            let center_top = top_y;
            let center_bottom = ch - bottom_pane_h;
            let center_h = (center_bottom - center_top).max(0);

            let mut left: Vec<&mut AuiPaneInfo> = self
                .panes
                .iter_mut()
                .filter(|p| p.shown && !p.floating && p.dock_side == AuiDockSide::Left)
                .collect();
            left.sort_by_key(|p| (p.layer, p.row));
            let mut left_x = 0i32;
            for p in &mut left {
                let w = docked_pane_width(p, side_caption);
                let rect = Rect::new(left_x, center_top, w as u32, center_h as u32);
                Self::platform_position_pane(art, frame_handle, parent, p, rect, &layout_child);
                left_x += w + padding;
            }

            let mut right: Vec<&mut AuiPaneInfo> = self
                .panes
                .iter_mut()
                .filter(|p| p.shown && !p.floating && p.dock_side == AuiDockSide::Right)
                .collect();
            right.sort_by_key(|p| (p.layer, p.row));
            let mut right_x = cw;
            let mut right_pane_w: i32 = 0;
            for p in &mut right {
                let w = docked_pane_width(p, side_caption);
                right_x -= w;
                let rect = Rect::new(right_x, center_top, w as u32, center_h as u32);
                Self::platform_position_pane(art, frame_handle, parent, p, rect, &layout_child);
                right_x -= padding;
                right_pane_w += w + padding;
            }
            let center_x = left_x;
            let center_w = (cw - left_x - right_pane_w).max(0);

            if let Some(idx) = self.center_pane_index {
                if let Some(p) = self.panes.get_mut(idx) {
                    if p.shown && !p.floating {
                        let rect =
                            Rect::new(center_x, center_top, center_w as u32, center_h as u32);
                        Self::platform_position_pane(
                            art,
                            frame_handle,
                            parent,
                            p,
                            rect,
                            &layout_child,
                        );
                    }
                }
            }

            for p in &self.panes {
                let visible = p.shown;
                if p.caption_visible {
                    if let Some(host) =
                        crate::platform::appkit::aui_pane_host_handle(frame_handle, &p.name)
                    {
                        crate::platform::appkit::set_view_visible(host, visible);
                        continue;
                    }
                }
                if let Some(child) = p.window_handle.filter(|&h| h != 0) {
                    crate::platform::appkit::set_view_visible(child, visible);
                }
            }

            self.register_pane_button_handlers(frame);
            self.register_pane_action_target(frame);
            Self::sync_sashes_after_layout(self, frame, parent);
        }

        #[cfg(target_os = "linux")]
        {
            let parent = frame.hwnd();
            if parent == 0 {
                return;
            }
            let frame_handle = parent;
            // Su Linux la dimensione client è già in pixel interi (i32).
            let (cw, ch) = crate::platform::gtk::frame_client_size(parent);
            let cw_u = cw.max(0) as u32;
            let padding = self.art.pane_padding() as i32;
            let caption = self.art.caption_height() as i32;
            let side_caption =
                crate::platform::gtk::aui_side_caption_width(self.art.caption_height()) as i32;
            let art = &self.art;

            let layout_child = |parent: crate::platform::NativeHandle,
                                child: crate::platform::NativeHandle,
                                rect: Rect| {
                crate::platform::gtk::set_view_frame(parent, child, rect);
            };

            let mut top: Vec<&mut AuiPaneInfo> = self
                .panes
                .iter_mut()
                .filter(|p| p.shown && !p.floating && p.dock_side == AuiDockSide::Top)
                .collect();
            top.sort_by_key(|p| (p.layer, p.row));
            let mut top_y = 0i32;
            for p in &mut top {
                let h = docked_pane_height(p, caption);
                let rect = Rect::new(0, top_y, cw_u, h as u32);
                Self::platform_position_pane(art, frame_handle, parent, p, rect, &layout_child);
                top_y += h + padding;
            }

            let mut bottom: Vec<&mut AuiPaneInfo> = self
                .panes
                .iter_mut()
                .filter(|p| p.shown && !p.floating && p.dock_side == AuiDockSide::Bottom)
                .collect();
            bottom.sort_by_key(|p| (p.layer, p.row));
            let mut bottom_y = ch;
            let mut bottom_pane_h: i32 = 0;
            for p in &mut bottom {
                let h = docked_pane_height(p, caption);
                bottom_y -= h;
                let rect = Rect::new(0, bottom_y, cw_u, h as u32);
                Self::platform_position_pane(art, frame_handle, parent, p, rect, &layout_child);
                bottom_y -= padding;
                bottom_pane_h += h + padding;
            }
            let center_top = top_y;
            let center_bottom = ch - bottom_pane_h;
            let center_h = (center_bottom - center_top).max(0);

            let mut left: Vec<&mut AuiPaneInfo> = self
                .panes
                .iter_mut()
                .filter(|p| p.shown && !p.floating && p.dock_side == AuiDockSide::Left)
                .collect();
            left.sort_by_key(|p| (p.layer, p.row));
            let mut left_x = 0i32;
            for p in &mut left {
                let w = docked_pane_width(p, side_caption);
                let rect = Rect::new(left_x, center_top, w as u32, center_h as u32);
                Self::platform_position_pane(art, frame_handle, parent, p, rect, &layout_child);
                left_x += w + padding;
            }

            let mut right: Vec<&mut AuiPaneInfo> = self
                .panes
                .iter_mut()
                .filter(|p| p.shown && !p.floating && p.dock_side == AuiDockSide::Right)
                .collect();
            right.sort_by_key(|p| (p.layer, p.row));
            let mut right_x = cw;
            let mut right_pane_w: i32 = 0;
            for p in &mut right {
                let w = docked_pane_width(p, side_caption);
                right_x -= w;
                let rect = Rect::new(right_x, center_top, w as u32, center_h as u32);
                Self::platform_position_pane(art, frame_handle, parent, p, rect, &layout_child);
                right_x -= padding;
                right_pane_w += w + padding;
            }
            let center_x = left_x;
            let center_w = (cw - left_x - right_pane_w).max(0);

            if let Some(idx) = self.center_pane_index {
                if let Some(p) = self.panes.get_mut(idx) {
                    if p.shown && !p.floating {
                        let rect =
                            Rect::new(center_x, center_top, center_w as u32, center_h as u32);
                        Self::platform_position_pane(
                            art,
                            frame_handle,
                            parent,
                            p,
                            rect,
                            &layout_child,
                        );
                    }
                }
            }

            for p in &self.panes {
                let visible = p.shown;
                if p.caption_visible {
                    if let Some(host) =
                        crate::platform::gtk::aui_pane_host_handle(frame_handle, &p.name)
                    {
                        crate::platform::gtk::set_view_visible(host, visible);
                        continue;
                    }
                }
                if let Some(child) = p.window_handle.filter(|&h| h != 0) {
                    crate::platform::gtk::set_view_visible(child, visible);
                }
            }

            self.register_pane_button_handlers(frame);
            self.register_pane_action_target(frame);
            Self::sync_sashes_after_layout(self, frame, parent);
        }

        #[cfg(all(
            not(target_os = "windows"),
            not(target_os = "macos"),
            not(target_os = "linux")
        ))]
        {
            let _ = frame;
        }
    }

    fn sync_sashes_after_layout(&self, frame: &Frame, parent: crate::platform::NativeHandle) {
        let specs = collect_sash_specs(&self.panes, self.art.sash(), self.art.sash_width());
        let frame_handle = frame.hwnd();
        #[cfg(target_os = "macos")]
        crate::platform::appkit::sync_aui_sashes(parent, frame_handle, &specs);
        #[cfg(target_os = "windows")]
        crate::platform::win32_aui::sync_aui_sashes(parent, frame_handle, &specs);
        #[cfg(target_os = "linux")]
        crate::platform::gtk::sync_aui_sashes(parent, frame_handle, &specs);
        #[cfg(all(
            not(target_os = "windows"),
            not(target_os = "macos"),
            not(target_os = "linux")
        ))]
        let _ = (parent, frame_handle, specs);
    }

    /// Return a perspective string describing the current dock
    /// layout. The string can be persisted to a config file and
    /// re-applied later with [`AuiManager::load_perspective`].
    pub fn save_perspective(&self) -> String {
        let mut out = String::new();
        for p in &self.panes {
            out.push_str(&format!(
                "name={}|caption={}|side={:?}|layer={}|row={}|best={}x{}|min={}x{}|shown={}|floating={}|pinned={}|auto_hide={}|auto_hidden={}\n",
                p.name,
                p.caption,
                p.dock_side,
                p.layer,
                p.row,
                p.best_size.0,
                p.best_size.1,
                p.min_size.0,
                p.min_size.1,
                p.shown,
                p.floating,
                p.pinned,
                p.auto_hide,
                p.auto_hidden,
            ));
        }
        out
    }

    /// Restore a perspective string previously produced by
    /// [`AuiManager::save_perspective`]. Panes that no longer exist
    /// are silently ignored; panes missing from the string are left
    /// untouched.
    pub fn load_perspective(&mut self, perspective: &str) {
        for line in perspective.lines() {
            let mut parts = line.split('|');
            let mut name = None;
            let mut side = None;
            let mut layer = 0;
            let mut row = 0;
            let mut best = (200, 200);
            let mut min = (50, 50);
            let mut shown = true;
            let mut floating = false;
            let mut pinned = true;
            let mut auto_hide = false;
            let mut auto_hidden = false;
            for part in &mut parts {
                if let Some(rest) = part.strip_prefix("name=") {
                    name = Some(rest.to_string());
                } else if let Some(rest) = part.strip_prefix("side=") {
                    side = match rest {
                        "Top" => Some(AuiDockSide::Top),
                        "Bottom" => Some(AuiDockSide::Bottom),
                        "Left" => Some(AuiDockSide::Left),
                        "Right" => Some(AuiDockSide::Right),
                        _ => Some(AuiDockSide::Floating),
                    };
                } else if let Some(rest) = part.strip_prefix("layer=") {
                    layer = rest.parse().unwrap_or(0);
                } else if let Some(rest) = part.strip_prefix("row=") {
                    row = rest.parse().unwrap_or(0);
                } else if let Some(rest) = part.strip_prefix("best=") {
                    let mut it = rest.split('x');
                    best = (
                        it.next().and_then(|s| s.parse().ok()).unwrap_or(200),
                        it.next().and_then(|s| s.parse().ok()).unwrap_or(200),
                    );
                } else if let Some(rest) = part.strip_prefix("min=") {
                    let mut it = rest.split('x');
                    min = (
                        it.next().and_then(|s| s.parse().ok()).unwrap_or(50),
                        it.next().and_then(|s| s.parse().ok()).unwrap_or(50),
                    );
                } else if let Some(rest) = part.strip_prefix("shown=") {
                    shown = rest == "true";
                } else if let Some(rest) = part.strip_prefix("floating=") {
                    floating = rest == "true";
                } else if let Some(rest) = part.strip_prefix("pinned=") {
                    pinned = rest == "true";
                } else if let Some(rest) = part.strip_prefix("auto_hide=") {
                    auto_hide = rest == "true";
                } else if let Some(rest) = part.strip_prefix("auto_hidden=") {
                    auto_hidden = rest == "true";
                }
            }
            if let (Some(name), Some(side)) = (name, side) {
                if let Some(p) = self.panes.iter_mut().find(|p| p.name == name) {
                    p.dock_side = side;
                    p.layer = layer;
                    p.row = row;
                    p.best_size = best;
                    p.min_size = min;
                    p.shown = shown;
                    p.floating = floating;
                    p.pinned = pinned;
                    p.auto_hide = auto_hide;
                    p.auto_hidden = auto_hidden;
                    if auto_hidden {
                        p.saved_best_size = best;
                    }
                }
            }
        }
    }
}

fn docked_pane_width(p: &AuiPaneInfo, caption: i32) -> i32 {
    if p.auto_hide && p.auto_hidden {
        return AUTO_HIDE_STRIP;
    }
    if p.caption_visible {
        (p.best_size.0 + caption).max(p.min_size.0)
    } else {
        p.best_size.0.max(p.min_size.0)
    }
}

fn docked_pane_height(p: &AuiPaneInfo, caption: i32) -> i32 {
    if p.auto_hide && p.auto_hidden {
        return AUTO_HIDE_STRIP;
    }
    if p.caption_visible {
        (p.best_size.1 + caption).max(p.min_size.1)
    } else {
        p.best_size.1.max(p.min_size.1)
    }
}

fn collect_sash_specs(panes: &[AuiPaneInfo], sash_color: u32, sash_width: u32) -> Vec<AuiSashSpec> {
    let sw = sash_width.max(1) as i32;
    let mut specs = Vec::new();
    for p in panes {
        if !p.shown || p.floating || p.auto_hidden {
            continue;
        }
        if p.rect.width == 0 && p.rect.height == 0 {
            continue;
        }
        match p.dock_side {
            AuiDockSide::Left => {
                let x = p.rect.x + p.rect.width as i32 - sw / 2;
                specs.push(AuiSashSpec {
                    pane_name: p.name.clone(),
                    vertical: true,
                    rect: Rect::new(x, p.rect.y, sw as u32, p.rect.height),
                    sash_color,
                });
            }
            AuiDockSide::Right => {
                let x = p.rect.x - sw / 2;
                specs.push(AuiSashSpec {
                    pane_name: p.name.clone(),
                    vertical: true,
                    rect: Rect::new(x.max(0), p.rect.y, sw as u32, p.rect.height),
                    sash_color,
                });
            }
            AuiDockSide::Top => {
                let y = p.rect.y + p.rect.height as i32 - sw / 2;
                specs.push(AuiSashSpec {
                    pane_name: p.name.clone(),
                    vertical: false,
                    rect: Rect::new(p.rect.x, y, p.rect.width, sw as u32),
                    sash_color,
                });
            }
            AuiDockSide::Bottom => {
                let y = p.rect.y - sw / 2;
                specs.push(AuiSashSpec {
                    pane_name: p.name.clone(),
                    vertical: false,
                    rect: Rect::new(p.rect.x, y.max(0), p.rect.width, sw as u32),
                    sash_color,
                });
            }
            _ => {}
        }
    }
    specs
}
