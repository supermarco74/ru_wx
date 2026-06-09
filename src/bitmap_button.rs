//! Bitmap-button control (`wxBitmapButton`).
//!
//! On Windows the widget is realised with the `BUTTON` common control
//! class using style `BS_BITMAP`. The button displays a bitmap
//! instead of a text label. Up to four state bitmaps can be
//! attached: the **label** bitmap (the default), the
//! **selected** bitmap (shown while the button is being held down),
//! the **disabled** bitmap (shown when the button is greyed out), and
//! the **focus** bitmap (shown when the button has keyboard focus).
//!
//! # Example
//! ```no_run
//! use ru_wx::bitmap_button::BitmapButton;
//! use ru_wx::bitmap::Bitmap;
//! use ru_wx::geometry::Colour;
//! use ru_wx::frame::Frame;
//!
//! let frame = Frame::builder().with_title("App").with_size(100, 100).build();
//! let bmp = Bitmap::new(32, 32);
//! let btn = BitmapButton::new(&frame, &bmp, 32, 32);
//! // Replace the displayed bitmap at runtime:
//! let new_bmp = Bitmap::new(32, 32);
//! btn.set_bitmap_label(&new_bmp);
//! ```

use std::cell::RefCell;
use std::rc::Rc;

use crate::bitmap::Bitmap;
use crate::frame::Frame;
use crate::geometry::Rect;
use crate::widget::{Widget, WidgetRef, Window};

#[cfg(target_os = "windows")]
use crate::platform::win32::{next_control_id, to_wide};
#[cfg(target_os = "windows")]
use windows_sys::Win32::Foundation::*;
#[cfg(target_os = "windows")]
use windows_sys::Win32::Graphics::Gdi::{DeleteObject, HBITMAP};
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::Input::KeyboardAndMouse::EnableWindow;
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::WindowsAndMessaging::*;

// ── Win32 constants (defined in <winuser.h>, not all exported by windows-sys 0.59) ──

/// `BS_BITMAP` — the button displays a bitmap.
#[cfg(target_os = "windows")]
const BS_BITMAP: u32 = 0x0080;

/// `BM_SETIMAGE` — associate an image (bitmap or icon) with the
/// button.
#[cfg(target_os = "windows")]
const BM_SETIMAGE: u32 = 0x00F7;
/// `BM_GETIMAGE` — return the image currently associated with the
/// button.
#[cfg(target_os = "windows")]
const BM_GETIMAGE: u32 = 0x00F6;

/// `IMAGE_BITMAP` — the image type passed to `BM_SETIMAGE` /
/// `BM_GETIMAGE` for bitmaps.
#[cfg(target_os = "windows")]
const IMAGE_BITMAP: usize = 0;

// ── Inner type ─────────────────────────────────────────────────────────

struct BitmapButtonInner {
    #[cfg(target_os = "windows")]
    hwnd: HWND,
    id: u16,
    rect: Rect,
    enabled: bool,
    visible: bool,
    /// Width of the bitmaps displayed by this button.
    bmp_width: i32,
    /// Height of the bitmaps displayed by this button.
    bmp_height: i32,
    /// `HBITMAP` handles for the four state bitmaps; 0 = "no
    /// bitmap for this state". The previous bitmap returned by
    /// `BM_SETIMAGE` is stored here so we can `DeleteObject` it
    /// when the user replaces it (or when the button is dropped).
    #[cfg(target_os = "windows")]
    hbitmap_label: isize,
    #[cfg(target_os = "windows")]
    hbitmap_selected: isize,
    #[cfg(target_os = "windows")]
    hbitmap_disabled: isize,
    #[cfg(target_os = "windows")]
    hbitmap_focus: isize,
}

#[derive(Clone)]
pub struct BitmapButton {
    inner: Rc<RefCell<BitmapButtonInner>>,
}

impl BitmapButton {
    /// Create a new bitmap button as a child of the given parent
    /// window, displaying the supplied bitmap at its native
    /// resolution.
    pub fn new<W: Window>(parent_in: &W, bitmap: &Bitmap, width: i32, height: i32) -> Self {
        let id = next_control_id();

        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        let hwnd = unsafe {
            let parent = parent_in.hwnd();
            let wide_class = to_wide("BUTTON");
            CreateWindowExW(
                0,
                wide_class.as_ptr(),
                std::ptr::null(), // BS_BITMAP: text is ignored
                WS_CHILD | WS_VISIBLE | BS_BITMAP,
                0,
                0,
                width,
                height,
                parent,
                id as usize as HMENU,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        };

        #[cfg(not(target_os = "windows"))]
        let _ = (parent_in, bitmap, width, height);

        let btn = BitmapButton {
            inner: Rc::new(RefCell::new(BitmapButtonInner {
                #[cfg(target_os = "windows")]
                hwnd,
                id,
                rect: Rect::new(0, 0, width as u32, height as u32),
                enabled: true,
                visible: true,
                bmp_width: width,
                bmp_height: height,
                #[cfg(target_os = "windows")]
                hbitmap_label: 0,
                #[cfg(target_os = "windows")]
                hbitmap_selected: 0,
                #[cfg(target_os = "windows")]
                hbitmap_disabled: 0,
                #[cfg(target_os = "windows")]
                hbitmap_focus: 0,
            })),
        };

        // Attach the supplied bitmap as the "label" bitmap.
        if !bitmap.is_null() {
            #[cfg(target_os = "windows")]
            unsafe {
                let hbm = bitmap.handle();
                let prev = SendMessageW(hwnd, BM_SETIMAGE, IMAGE_BITMAP, hbm as isize) as isize;
                btn.inner.borrow_mut().hbitmap_label = hbm as isize;
                // If there was a previous bitmap, delete it.
                if prev != 0 {
                    DeleteObject(prev as HBITMAP);
                }
            }
            #[cfg(not(target_os = "windows"))]
            let _ = bitmap;
        }

        btn
    }

    /// Create a bitmap button from an SVG file (rasterised to
    /// `width × height` pixels).
    pub fn new_from_svg<W: Window>(
        parent_in: &W,
        svg_path: &std::path::Path,
        width: i32,
        height: i32,
    ) -> Self {
        let id = next_control_id();

        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        let (hwnd, hbitmap) = unsafe {
            let parent = parent_in.hwnd();
            let wide_class = to_wide("BUTTON");
            let hwnd = CreateWindowExW(
                0,
                wide_class.as_ptr(),
                std::ptr::null(),
                WS_CHILD | WS_VISIBLE | BS_BITMAP,
                0,
                0,
                width,
                height,
                parent,
                id as usize as HMENU,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            );
            let hbitmap = crate::icon::load_svg_as_hbitmap(svg_path, width as u32, height as u32);
            if let Some(hbmp) = hbitmap {
                SendMessageW(hwnd, BM_SETIMAGE, IMAGE_BITMAP, hbmp as isize);
            }
            (hwnd, hbitmap)
        };

        #[cfg(not(target_os = "windows"))]
        let _ = (parent_in, svg_path, width, height);

        BitmapButton {
            inner: Rc::new(RefCell::new(BitmapButtonInner {
                #[cfg(target_os = "windows")]
                hwnd,
                id,
                rect: Rect::new(0, 0, width as u32, height as u32),
                enabled: true,
                visible: true,
                bmp_width: width,
                bmp_height: height,
                #[cfg(target_os = "windows")]
                hbitmap_label: hbitmap.map(|h| h as isize).unwrap_or(0),
                #[cfg(target_os = "windows")]
                hbitmap_selected: 0,
                #[cfg(target_os = "windows")]
                hbitmap_disabled: 0,
                #[cfg(target_os = "windows")]
                hbitmap_focus: 0,
            })),
        }
    }

    /// Create a bitmap button from embedded SVG bytes.
    pub fn new_from_svg_bytes<W: Window>(
        parent_in: &W,
        svg_bytes: &[u8],
        width: i32,
        height: i32,
    ) -> Self {
        let id = next_control_id();

        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        let (hwnd, hbitmap) = unsafe {
            let parent = parent_in.hwnd();
            let wide_class = to_wide("BUTTON");
            let hwnd = CreateWindowExW(
                0,
                wide_class.as_ptr(),
                std::ptr::null(),
                WS_CHILD | WS_VISIBLE | BS_BITMAP,
                0,
                0,
                width,
                height,
                parent,
                id as usize as HMENU,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            );
            let hbitmap =
                crate::icon::svg_bytes_to_hbitmap(svg_bytes, width as u32, height as u32);
            if let Some(hbmp) = hbitmap {
                SendMessageW(hwnd, BM_SETIMAGE, IMAGE_BITMAP, hbmp as isize);
            }
            (hwnd, hbitmap)
        };

        #[cfg(not(target_os = "windows"))]
        let _ = (parent_in, svg_bytes, width, height);

        BitmapButton {
            inner: Rc::new(RefCell::new(BitmapButtonInner {
                #[cfg(target_os = "windows")]
                hwnd,
                id,
                rect: Rect::new(0, 0, width as u32, height as u32),
                enabled: true,
                visible: true,
                bmp_width: width,
                bmp_height: height,
                #[cfg(target_os = "windows")]
                hbitmap_label: hbitmap.map(|h| h as isize).unwrap_or(0),
                #[cfg(target_os = "windows")]
                hbitmap_selected: 0,
                #[cfg(target_os = "windows")]
                hbitmap_disabled: 0,
                #[cfg(target_os = "windows")]
                hbitmap_focus: 0,
            })),
        }
    }

    /// Set the bitmap displayed when the button is in its default
    /// (not pressed, not disabled, no keyboard focus) state.
    /// Replaces any previously-assigned label bitmap; the old
    /// bitmap is freed with `DeleteObject`.
    pub fn set_bitmap_label(&self, bitmap: &Bitmap) {
        if bitmap.is_null() {
            return;
        }
        #[cfg(target_os = "windows")]
        unsafe {
            let hwnd = self.inner.borrow().hwnd;
            let hbm = bitmap.handle();
            let prev = SendMessageW(hwnd, BM_SETIMAGE, IMAGE_BITMAP, hbm as isize) as isize;
            self.inner.borrow_mut().hbitmap_label = hbm as isize;
            if prev != 0 {
                DeleteObject(prev as HBITMAP);
            }
        }
        #[cfg(not(target_os = "windows"))]
        let _ = bitmap;
    }

    /// Get the control ID (used for WM_COMMAND dispatch).
    pub fn id(&self) -> u16 {
        self.inner.borrow().id
    }

    /// Set a click callback. Must register with the parent frame.
    pub fn on_click<F: FnMut() + 'static>(&self, frame: &Frame, callback: F) {
        let id = self.inner.borrow().id;
        frame.register_command_handler(id, Box::new(callback));
    }

    /// Return the bitmap width in pixels.
    pub fn bitmap_width(&self) -> i32 {
        self.inner.borrow().bmp_width
    }

    /// Return the bitmap height in pixels.
    pub fn bitmap_height(&self) -> i32 {
        self.inner.borrow().bmp_height
    }

    /// Get a `WidgetRef` for use with sizers.
    pub fn as_widget_ref(&self) -> WidgetRef {
        self.inner.clone()
    }
}

impl Widget for BitmapButtonInner {
    fn native_handle(&self) -> isize {
        #[cfg(target_os = "windows")]
        {
            self.hwnd as isize
        }
        #[cfg(not(target_os = "windows"))]
        0
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
        self.enabled
    }

    fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            EnableWindow(self.hwnd, if enabled { 1 } else { 0 });
        }
    }
}

impl Drop for BitmapButtonInner {
    fn drop(&mut self) {
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments. Each
        // bitmap, if non-zero, was either returned by
        // `CreateDIBSection` (via `Bitmap::new`) or by `BM_SETIMAGE`
        // (the previous bitmap, which we own because we replaced
        // it).
        unsafe {
            for hbm in [
                self.hbitmap_label,
                self.hbitmap_selected,
                self.hbitmap_disabled,
                self.hbitmap_focus,
            ]
            .iter()
            .copied()
            .filter(|&h| h != 0)
            {
                DeleteObject(hbm as HBITMAP);
            }
        }
    }
}

// ── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// The Win32 constants we re-export must match the values in
    /// `<winuser.h>`. A regression here would silently break
    /// `BM_SETIMAGE` dispatch (a wrong `BS_BITMAP` would make the
    /// control render text instead of a bitmap).
    #[cfg(target_os = "windows")]
    #[test]
    fn win32_constants_pinned() {
        assert_eq!(BS_BITMAP, 0x0080);
        assert_eq!(BM_SETIMAGE, 0x00F7);
        assert_eq!(BM_GETIMAGE, 0x00F6);
        assert_eq!(IMAGE_BITMAP, 0);
    }
}
