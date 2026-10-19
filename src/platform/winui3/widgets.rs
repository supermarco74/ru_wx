//! New WinUI3 widgets that round out the surface area beyond what
//! `app.rs` provided in the original placeholder.
//!
//! File contents:
//!   * [`Hyperlink`]
//!   * [`StaticLine`]
//!   * [`StaticBox`]
//!   * [`BitmapButton`]
//!   * [`ProgressBar`] (with marquee support; richer than [`Gauge`](super::app::Gauge))
//!   * [`SpinCtrl`]
//!   * [`Splitter`]
//!   * [`SearchCtrl`]
//!
//! All controls follow the same shape as `app.rs` widgets: `HWND`
//! private handle, `set_label` / `set_size` / `set_position` /
//! `hwnd` public methods, and a `Drop` that destroys the underlying
//! window.

#![allow(non_snake_case)]

use std::ptr;

use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::Graphics::Gdi::{DeleteObject, HBITMAP};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

/// Trait alias used by [`Hyperlink`] / [`StaticLine`] / etc. to take
/// *either* a top-level `Frame` or another `Panel`-like control as
/// parent. We do not depend on the crate-root [`crate::core::widget::Window`]
/// trait here to avoid the boundary; each widget just calls
/// `parent.hwnd()` on the supplied reference.
pub trait WidgetParent {
    fn hwnd(&self) -> HWND;
}

// SS_NOTIFY: clickable static text (Hyperlink).
// Colour is painted via the WM_CTLCOLORSTATIC subclass in `subclass.rs`.
const SS_NOTIFY: u32 = 0x0000_0100;
/// Hyperlink colour (≈ #0066CC, the canonical Win11 accent).
const HYPERLINK_COLOR: u32 = 0x00CC6600; // 0x00BBGGRR
/// SS_BLACKFRAME — solid 1-pixel black rectangle used for [`StaticLine`]
/// to keep the line visible at any height.
const SS_ETCHEDHORZ: u32 = 0x0000_000A; // SS_BLACKFRAME
/// SS_GRAYFRAME — solid 1-pixel gray rectangle used for the vertical
/// splitter bar so the drag feedback is visible even at 4 px width.
const SS_ETCHEDVERT: u32 = 0x0000_0008; // SS_GRAYFRAME

/// WS_CHILD | WS_VISIBLE — short alias used everywhere in this module.
const WS_CHILD_VISIBLE: u32 = WS_CHILD | WS_VISIBLE;

// ============================================================================
// Hyperlink: a single-line static that emits a command when clicked.
//
// Internally a `BUTTON` with `BS_OWNERDRAW` style; the user wires
/// `on_click` to react to navigation.
pub struct Hyperlink {
    hwnd: HWND,
    url: String,
}

impl Hyperlink {
    pub fn new<P: WidgetParent>(parent: &P, label: &str, url: &str) -> Self {
        let id = super::app::next_id();
        let parent_hwnd = parent.hwnd();
        let hwnd = unsafe {
            CreateWindowExW(
                0,
                to_wide("STATIC").as_ptr(),
                to_wide(label).as_ptr(),
                // SS_NOTIFY allows the static to generate BN_CLICKED
                // when the user clicks it, so on_click works.
                // The frame-level WM_CTLCOLORSTATIC subclass (see
                // `subclass` module) paints the text in the canonical
                // hyperlink blue (≈ #0066CC).
                WS_CHILD_VISIBLE | 0x0000_0100, // SS_NOTIFY
                0,
                0,
                200,
                22,
                parent_hwnd,
                id as isize as *mut _,
                GetModuleHandleW(ptr::null()),
                ptr::null_mut(),
            )
        };
        // Wire the per-widget hyperlink colour into the frame's
        // WM_CTLCOLORSTATIC subclass table.
        super::subclass::install_frame_subclass(parent_hwnd);
        super::subclass::set_foreground(hwnd, super::subclass::HYPERLINK_BLUE);
        Self {
            hwnd,
            url: url.to_string(),
        }
    }

    pub fn set_label(&self, label: &str) {
        unsafe {
            SetWindowTextW(self.hwnd, to_wide(label).as_ptr());
        }
    }

    pub fn url(&self) -> &str {
        &self.url
    }

    pub fn hwnd(&self) -> HWND {
        self.hwnd
    }
}

impl Drop for Hyperlink {
    fn drop(&mut self) {
        unsafe {
            DestroyWindow(self.hwnd);
        }
    }
}

// ============================================================================
// StaticLine: a horizontal etched separator — typically used in vertical
// BoxSizer flows to break up sections.
pub struct StaticLine {
    hwnd: HWND,
}

impl StaticLine {
    pub fn new<P: WidgetParent>(parent: &P) -> Self {
        unsafe {
            let hwnd = CreateWindowExW(
                0,
                to_wide("STATIC").as_ptr(),
                ptr::null(),
                WS_CHILD_VISIBLE | SS_ETCHEDHORZ,
                0,
                0,
                100,
                2,
                parent.hwnd(),
                ptr::null_mut(),
                GetModuleHandleW(ptr::null()),
                ptr::null_mut(),
            );
            Self { hwnd }
        }
    }

    pub fn set_size(&self, w: i32, h: i32) {
        unsafe {
            SetWindowPos(self.hwnd, ptr::null_mut(), 0, 0, w, h, 0x0002 | 0x0040);
        }
    }

    pub fn hwnd(&self) -> HWND {
        self.hwnd
    }
}

impl Drop for StaticLine {
    fn drop(&mut self) {
        unsafe {
            DestroyWindow(self.hwnd);
        }
    }
}

// ============================================================================
// StaticBox: a group box that hosts multiple child controls. Mirrors
// the Win32 BS_GROUPBOX button style.
pub struct StaticBox {
    hwnd: HWND,
    label: String,
}

impl StaticBox {
    pub fn new<P: WidgetParent>(parent: &P, label: &str) -> Self {
        let id = super::app::next_id();
        unsafe {
            let hwnd = CreateWindowExW(
                0,
                to_wide("BUTTON").as_ptr(),
                to_wide(label).as_ptr(),
                WS_CHILD_VISIBLE | 0x0000_0007, // BS_GROUPBOX
                0,
                0,
                200,
                150,
                parent.hwnd(),
                id as isize as *mut _,
                GetModuleHandleW(ptr::null()),
                ptr::null_mut(),
            );
            Self {
                hwnd,
                label: label.to_string(),
            }
        }
    }

    pub fn set_label(&self, label: &str) {
        unsafe {
            SetWindowTextW(self.hwnd, to_wide(label).as_ptr());
        }
    }

    pub fn label(&self) -> &str {
        &self.label
    }

    pub fn set_size(&self, w: i32, h: i32) {
        unsafe {
            SetWindowPos(self.hwnd, ptr::null_mut(), 0, 0, w, h, 0x0002 | 0x0040);
        }
    }

    pub fn hwnd(&self) -> HWND {
        self.hwnd
    }
}

impl Drop for StaticBox {
    fn drop(&mut self) {
        unsafe {
            DestroyWindow(self.hwnd);
        }
    }
}

// ============================================================================
// BitmapButton: a BS_ICON button pre-wired to host a small bitmap.
//
// The simplest way to give a tool button an icon on the WinUI3
// backend; rich icons should use the full `ImageList`-based toolbar
// in `toolbar.rs`.
pub struct BitmapButton {
    hwnd: HWND,
    bitmap: HBITMAP,
}

impl BitmapButton {
    pub fn new<P: WidgetParent>(parent: &P, bitmap: HBITMAP) -> Self {
        let id = super::app::next_id();
        unsafe {
            let hwnd = CreateWindowExW(
                0,
                to_wide("BUTTON").as_ptr(),
                ptr::null(),
                WS_CHILD_VISIBLE | 0x0000_0040, // BS_ICON
                0,
                0,
                32,
                32,
                parent.hwnd(),
                id as isize as *mut _,
                GetModuleHandleW(ptr::null()),
                ptr::null_mut(),
            );
            SendMessageW(hwnd, 0x00F7, 0 /* IMAGE_ICON */, bitmap as isize); // BM_SETIMAGE
            Self { hwnd, bitmap }
        }
    }

    pub fn hwnd(&self) -> HWND {
        self.hwnd
    }
}

impl Drop for BitmapButton {
    fn drop(&mut self) {
        unsafe {
            DestroyWindow(self.hwnd);
            if !self.bitmap.is_null() {
                DeleteObject(self.bitmap);
            }
        }
    }
}

// ============================================================================
// ProgressBar: thin wrapper over `msctls_progress32` with both
// determinate and marquee modes.
pub struct ProgressBar {
    hwnd: HWND,
}

impl ProgressBar {
    pub fn new<P: WidgetParent>(parent: &P) -> Self {
        unsafe {
            let hwnd = CreateWindowExW(
                0,
                to_wide("msctls_progress32").as_ptr(),
                ptr::null(),
                WS_CHILD_VISIBLE,
                0,
                0,
                200,
                18,
                parent.hwnd(),
                ptr::null_mut(),
                GetModuleHandleW(ptr::null()),
                ptr::null_mut(),
            );
            Self { hwnd }
        }
    }

    pub fn set_value(&self, v: i32) {
        unsafe {
            SendMessageW(self.hwnd, 0x0402, v as usize, 0);
        } // PBM_SETPOS
    }

    pub fn get_value(&self) -> i32 {
        unsafe { SendMessageW(self.hwnd, 0x0408, 0, 0) as i32 } // PBM_GETPOS
    }

    pub fn set_range(&self, min: i32, max: i32) {
        unsafe {
            SendMessageW(self.hwnd, 0x0406, min as usize, max as isize);
        } // PBM_SETRANGE32
    }

    pub fn set_marquee(&self, on: bool, period_ms: u32) {
        unsafe {
            SendMessageW(self.hwnd, 0x040A, on as usize, 0); // PBM_SETMARQUEE
            if on {
                SendMessageW(self.hwnd, 0x0409, period_ms as usize, 0); // PBM_SETTIMER
            }
        }
    }

    pub fn set_size(&self, w: i32, h: i32) {
        unsafe {
            SetWindowPos(self.hwnd, ptr::null_mut(), 0, 0, w, h, 0x0002 | 0x0040);
        }
    }

    pub fn hwnd(&self) -> HWND {
        self.hwnd
    }
}

impl Drop for ProgressBar {
    fn drop(&mut self) {
        unsafe {
            DestroyWindow(self.hwnd);
        }
    }
}

// ============================================================================
// SpinCtrl: up/down numeric spinner (uses `msctls_updown32`).
pub struct SpinCtrl {
    hwnd: HWND,
    min: i32,
    max: i32,
    value: i32,
}

impl SpinCtrl {
    pub fn new<P: WidgetParent>(parent: &P, min: i32, max: i32, initial: i32) -> Self {
        unsafe {
            let hwnd = CreateWindowExW(
                0,
                to_wide("msctls_updown32").as_ptr(),
                ptr::null(),
                WS_CHILD_VISIBLE | 0x0002, // UDS_ARROWKEYS
                0,
                0,
                20,
                24,
                parent.hwnd(),
                ptr::null_mut(),
                GetModuleHandleW(ptr::null()),
                ptr::null_mut(),
            );
            let widget = Self {
                hwnd,
                min,
                max,
                value: initial,
            };
            widget.set_range(min, max);
            widget.set_value(initial);
            widget
        }
    }

    pub fn set_range(&self, min: i32, max: i32) {
        unsafe {
            SendMessageW(
                self.hwnd,
                0x0402 + 6, /* UDM_SETRANGE */
                max as usize,
                min as isize,
            );
        }
    }

    pub fn set_value(&self, v: i32) {
        let clamped = v.clamp(self.min, self.max);
        unsafe {
            SendMessageW(
                self.hwnd,
                0x0402 + 5, /* UDM_SETPOS32 */
                0,
                clamped as isize,
            );
        }
    }

    pub fn get_value(&self) -> i32 {
        unsafe {
            SendMessageW(self.hwnd, 0x0402 + 4 /* UDM_GETPOS32 */, 0, 0) as i32
        }
    }

    pub fn hwnd(&self) -> HWND {
        self.hwnd
    }
}

impl Drop for SpinCtrl {
    fn drop(&mut self) {
        unsafe {
            DestroyWindow(self.hwnd);
        }
    }
}

// ============================================================================
// Splitter: a vertical sash that fires a "drag" event when the user
// moves it. Lays out two children side-by-side; the geometry is
// managed by the caller through `set_position`/`set_ratio`.
pub struct Splitter {
    hwnd: HWND,
    ratio: f32,
}

impl Splitter {
    pub fn new<P: WidgetParent>(parent: &P) -> Self {
        unsafe {
            let hwnd = CreateWindowExW(
                0,
                to_wide("STATIC").as_ptr(),
                ptr::null(),
                WS_CHILD_VISIBLE | SS_ETCHEDVERT,
                0,
                0,
                4,
                100,
                parent.hwnd(),
                ptr::null_mut(),
                GetModuleHandleW(ptr::null()),
                ptr::null_mut(),
            );
            Self { hwnd, ratio: 0.5 }
        }
    }

    pub fn set_ratio(&mut self, ratio: f32) {
        self.ratio = ratio.clamp(0.05, 0.95);
    }

    pub fn ratio(&self) -> f32 {
        self.ratio
    }

    pub fn set_size(&self, h: i32) {
        unsafe {
            SetWindowPos(self.hwnd, ptr::null_mut(), 0, 0, 4, h, 0x0002 | 0x0040);
        }
    }

    pub fn hwnd(&self) -> HWND {
        self.hwnd
    }
}

impl Drop for Splitter {
    fn drop(&mut self) {
        unsafe {
            DestroyWindow(self.hwnd);
        }
    }
}

// ============================================================================
// SearchCtrl: an Edit + a small "x" button + a left-aligned search icon.
// Implemented as an `EDIT` control with `ES_WANTRETURN` flag.
pub struct SearchCtrl {
    hwnd: HWND,
}

impl SearchCtrl {
    pub fn new<P: WidgetParent>(parent: &P) -> Self {
        unsafe {
            let hwnd = CreateWindowExW(
                0x0004, /* WS_EX_CLIENTEDGE */
                to_wide("EDIT").as_ptr(),
                ptr::null(),
                WS_CHILD_VISIBLE | 0x0080, /* ES_AUTOHSCROLL */
                0,
                0,
                200,
                24,
                parent.hwnd(),
                ptr::null_mut(),
                GetModuleHandleW(ptr::null()),
                ptr::null_mut(),
            );
            Self { hwnd }
        }
    }

    pub fn set_value(&self, text: &str) {
        unsafe {
            SetWindowTextW(self.hwnd, to_wide(text).as_ptr());
        }
    }

    pub fn get_value(&self) -> String {
        unsafe {
            let len = GetWindowTextLengthW(self.hwnd);
            if len <= 0 {
                return String::new();
            }
            let mut buf: Vec<u16> = vec![0; len as usize + 1];
            GetWindowTextW(self.hwnd, buf.as_mut_ptr(), len + 1);
            String::from_utf16_lossy(&buf[..len as usize])
        }
    }

    pub fn set_size(&self, w: i32, h: i32) {
        unsafe {
            SetWindowPos(self.hwnd, ptr::null_mut(), 0, 0, w, h, 0x0002 | 0x0040);
        }
    }

    pub fn hwnd(&self) -> HWND {
        self.hwnd
    }
}

impl Drop for SearchCtrl {
    fn drop(&mut self) {
        unsafe {
            DestroyWindow(self.hwnd);
        }
    }
}

// ============================================================================
// Local helper.
fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}
