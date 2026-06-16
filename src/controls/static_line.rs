//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Horizontal / vertical line separator (`wxStaticLine`).
//!
//! On Windows this is a `STATIC` child with style `SS_ETCHEDHORZ`
//! (default) or `SS_ETCHEDVERT`. It draws an etched 3D line, does not
//! accept focus, and ignores mouse / keyboard input.
//!
//! Use [`StaticLine::new_horizontal`] / [`StaticLine::new_vertical`]
//! (or the generic [`StaticLine::new`] with a
//! [`StaticLineOrientation`]). Width and height are managed by the
//! parent sizer just like any other widget.

use std::cell::RefCell;
use std::rc::Rc;

use crate::core::geometry::Rect;
use crate::core::widget::{Widget, WidgetRef, Window};

use crate::platform::next_control_id;
#[cfg(target_os = "windows")]
use crate::platform::win32::to_wide;
#[cfg(target_os = "windows")]
use windows_sys::Win32::Foundation::*;
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::WindowsAndMessaging::*;

// `STATIC` class etched-line styles (not exposed by `windows-sys 0.59`,
// defined as raw constants — same values as the C++ `<winuser.h>`).
#[cfg(target_os = "windows")]
const SS_ETCHEDHORZ: u32 = 0x0010;
#[cfg(target_os = "windows")]
const SS_ETCHEDVERT: u32 = 0x0011;

/// Direction of a [`StaticLine`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum StaticLineOrientation {
    /// Horizontal line (default; ~2 px tall).
    #[default]
    Horizontal,
    /// Vertical line (~2 px wide).
    Vertical,
}

struct StaticLineInner {
    #[cfg(target_os = "windows")]
    hwnd: HWND,
    orientation: StaticLineOrientation,
    rect: Rect,
    visible: bool,
}

#[derive(Clone)]
pub struct StaticLine {
    inner: Rc<RefCell<StaticLineInner>>,
}

impl StaticLine {
    /// Create a horizontal `StaticLine` as a child of the given parent.
    pub fn new_horizontal<W: Window>(parent_in: &W) -> Self {
        Self::new(parent_in, StaticLineOrientation::Horizontal)
    }

    /// Create a vertical `StaticLine` as a child of the given parent.
    pub fn new_vertical<W: Window>(parent_in: &W) -> Self {
        Self::new(parent_in, StaticLineOrientation::Vertical)
    }

    /// Create a `StaticLine` with an explicit orientation.
    pub fn new<W: Window>(parent_in: &W, orientation: StaticLineOrientation) -> Self {
        let id = next_control_id();

        // Default size: wide & thin for horizontal, tall & thin for
        // vertical. The parent sizer will overwrite these on layout.
        let (default_w, default_h): (u32, u32) = match orientation {
            StaticLineOrientation::Horizontal => (200, 2),
            StaticLineOrientation::Vertical => (2, 200),
        };

        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        let hwnd = unsafe {
            let parent = parent_in.hwnd();
            let wide_class = to_wide("STATIC");
            let line_style: u32 = match orientation {
                StaticLineOrientation::Horizontal => SS_ETCHEDHORZ,
                StaticLineOrientation::Vertical => SS_ETCHEDVERT,
            };
            CreateWindowExW(
                0,
                wide_class.as_ptr(),
                std::ptr::null(),
                WS_CHILD | WS_VISIBLE | line_style,
                0,
                0,
                default_w as i32,
                default_h as i32,
                parent,
                id as usize as HMENU,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        };

        #[cfg(not(target_os = "windows"))]
        let _ = (parent_in, default_w, default_h);

        StaticLine {
            inner: Rc::new(RefCell::new(StaticLineInner {
                #[cfg(target_os = "windows")]
                hwnd,
                orientation,
                rect: Rect::new(0, 0, default_w, default_h),
                visible: true,
            })),
        }
    }

    /// Return the line orientation.
    pub fn orientation(&self) -> StaticLineOrientation {
        self.inner.borrow().orientation
    }

    /// Get a `WidgetRef` for use with sizers.
    pub fn as_widget_ref(&self) -> WidgetRef {
        self.inner.clone()
    }

    /// Return the native window handle (HWND on Windows, 0 elsewhere).
    #[cfg(target_os = "windows")]
    pub fn hwnd(&self) -> HWND {
        self.inner.borrow().hwnd
    }
    #[cfg(not(target_os = "windows"))]
    pub fn hwnd(&self) -> isize {
        0
    }
}

#[cfg(target_os = "windows")]
impl Window for StaticLine {
    fn hwnd(&self) -> HWND {
        self.hwnd()
    }
}

impl Widget for StaticLineInner {
    fn native_handle(&self) -> isize {
        #[cfg(target_os = "windows")]
        {
            self.hwnd as isize
        }
        #[cfg(not(target_os = "windows"))]
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
    }

    fn set_size(&mut self, w: u32, h: u32) {
        self.rect.width = w;
        self.rect.height = h;
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            MoveWindow(self.hwnd, self.rect.x, self.rect.y, w as i32, h as i32, 1);
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
    }

    fn is_enabled(&self) -> bool {
        // StaticLine has no enabled state.
        true
    }

    fn set_enabled(&mut self, _enabled: bool) {
        // StaticLine has no enabled state.
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_orientation_is_horizontal() {
        assert_eq!(StaticLineOrientation::default(), StaticLineOrientation::Horizontal);
    }

    #[test]
    fn orientations_compare_distinctly() {
        let h = StaticLineOrientation::Horizontal;
        let v = StaticLineOrientation::Vertical;
        assert_ne!(h, v);
        // Copy / Clone / Eq / Hash
        let _h2 = h;
        let _v2 = v;
        assert_eq!(h, StaticLineOrientation::Horizontal);
    }
}
