//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Read-only image control (`wxStaticBitmap`).
//!
//! On Windows the widget is a `STATIC` child with style `SS_BITMAP` (or
//! `SS_ICON` for HICON content). The image is applied via the
//! `STM_SETIMAGE` message, and a `SS_REALSIZECONTROL` style (when
//! available) is OR-ed in so the control sizes itself to the bitmap
//! unless the parent sizer forces a different size.
//!
//! Use [`StaticBitmap::new`] to create a control bound to a
//! [`crate::BitmapBundle`], [`StaticBitmap::with_bitmap`] for a
//! single-resolution `HBITMAP`, or [`StaticBitmap::with_icon`] for an
//! `HICON` (e.g. one created from an SVG).
//!
//! The widget stores a *copy* of the bundle/best-fit bitmap (it does
//! not own the caller's handle); when a new image is set or the
//! control is dropped, the previously held bitmap (if any) is
//! `DeleteObject`-released.

use std::cell::RefCell;
use std::rc::Rc;

use crate::dc::bitmap_bundle::{BitmapBundle, RawBitmap};
use crate::core::geometry::Rect;
use crate::core::widget::{Widget, WidgetRef, Window};

#[cfg(target_os = "windows")]
use crate::platform::win32::{next_control_id, to_wide};
#[cfg(target_os = "windows")]
use windows_sys::Win32::Foundation::*;
#[cfg(target_os = "windows")]
use windows_sys::Win32::Graphics::Gdi::{
    DeleteObject, BITMAP, GetObjectW, HBITMAP,
};
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::Input::KeyboardAndMouse::EnableWindow;
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::WindowsAndMessaging::*;

// ── Win32 constants not always exposed by `windows-sys 0.59` ───────────

#[cfg(target_os = "windows")]
const SS_BITMAP: u32 = 0x000E;
#[cfg(target_os = "windows")]
#[allow(dead_code)]
const SS_ICON: u32 = 0x0003;
#[cfg(target_os = "windows")]
const SS_CENTERIMAGE: u32 = 0x0200;
#[cfg(target_os = "windows")]
#[allow(dead_code)]
const SS_REALSIZECONTROL: u32 = 0x0800;

// `STM_SETIMAGE` (a.k.a. `BM_SETIMAGE` on `STATIC` controls)
#[cfg(target_os = "windows")]
const STM_SETIMAGE: u32 = 0x0172;
#[cfg(target_os = "windows")]
#[allow(dead_code)]
const STM_GETIMAGE: u32 = 0x0173;

// `wParam` values for `STM_SETIMAGE`
#[cfg(target_os = "windows")]
const IMAGE_BITMAP: usize = 0;
#[cfg(target_os = "windows")]
const IMAGE_ICON: usize = 1;

/// What kind of image the control is currently displaying.
#[cfg(target_os = "windows")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StaticBitmapImageKind {
    None,
    Bitmap,
    Icon,
}

#[cfg(not(target_os = "windows"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StaticBitmapImageKind {
    None,
    Bitmap,
    Icon,
}

struct StaticBitmapInner {
    #[cfg(target_os = "windows")]
    hwnd: HWND,
    /// Native handle we currently own and must release on drop / replace.
    /// `0` means "no image set".
    current_handle: isize,
    image_kind: StaticBitmapImageKind,
    rect: Rect,
    visible: bool,
    enabled: bool,
}

#[derive(Clone)]
pub struct StaticBitmap {
    inner: Rc<RefCell<StaticBitmapInner>>,
}

impl StaticBitmap {
    /// Default size used when the caller has not provided an explicit
    /// one. Matched to the typical 16-px toolbar icon; sizers usually
    /// override it.
    const DEFAULT_W: u32 = 16;
    const DEFAULT_H: u32 = 16;

    /// Create a `StaticBitmap` bound to a [`BitmapBundle`]. The bundle
    /// is queried for the best fit at the requested `size`; if the
    /// bundle is empty the control is created with no image.
    pub fn new<W: Window>(
        parent_in: &W,
        bundle: &BitmapBundle,
        size: (u32, u32),
    ) -> Self {
        let slf = Self::with_size(parent_in, size.0.max(Self::DEFAULT_W), size.1.max(Self::DEFAULT_H));
        if let Some(bmp) = bundle.best_for_size(size) {
            slf.set_bitmap(bmp);
        }
        slf
    }

    /// Create a `StaticBitmap` for a single `HBITMAP` (Windows-only).
    #[cfg(target_os = "windows")]
    pub fn with_bitmap<W: Window>(parent_in: &W, hbitmap: HBITMAP, width: u32, height: u32) -> Self {
        let slf = Self::with_size(parent_in, width, height);
        slf.set_raw_bitmap(hbitmap);
        slf
    }

    /// Non-Windows stub for `with_bitmap`.
    #[cfg(not(target_os = "windows"))]
    pub fn with_bitmap<W: Window>(
        parent_in: &W,
        _hbitmap: (),
        width: u32,
        height: u32,
    ) -> Self {
        let _ = parent_in;
        Self::with_size_stub(width, height)
    }

    /// Create a `StaticBitmap` for an `HICON` (Windows-only).
    #[cfg(target_os = "windows")]
    pub fn with_icon<W: Window>(parent_in: &W, hicon: HICON, size: (u32, u32)) -> Self {
        let slf = Self::with_size(parent_in, size.0.max(Self::DEFAULT_W), size.1.max(Self::DEFAULT_H));
        slf.set_icon(hicon);
        slf
    }

    /// Non-Windows stub for `with_icon`.
    #[cfg(not(target_os = "windows"))]
    pub fn with_icon<W: Window>(
        parent_in: &W,
        _hicon: (),
        size: (u32, u32),
    ) -> Self {
        let _ = parent_in;
        Self::with_size_stub(size.0, size.1)
    }

    /// Create an empty `StaticBitmap` (no image). The control is sized
    /// to the given width/height; you can set the image later with
    /// [`set_bitmap`](Self::set_bitmap).
    pub fn with_size<W: Window>(parent_in: &W, width: u32, height: u32) -> Self {
        let id = next_control_id();

        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        let hwnd = unsafe {
            let parent = parent_in.hwnd();
            let wide_class = to_wide("STATIC");
            CreateWindowExW(
                0,
                wide_class.as_ptr(),
                std::ptr::null(),
                WS_CHILD | WS_VISIBLE | SS_BITMAP | SS_CENTERIMAGE,
                0,
                0,
                width as i32,
                height as i32,
                parent,
                id as usize as HMENU,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        };

        #[cfg(not(target_os = "windows"))]
        let _ = (parent_in, width, height);

        StaticBitmap {
            inner: Rc::new(RefCell::new(StaticBitmapInner {
                #[cfg(target_os = "windows")]
                hwnd,
                current_handle: 0,
                image_kind: StaticBitmapImageKind::None,
                rect: Rect::new(0, 0, width, height),
                visible: true,
                enabled: true,
            })),
        }
    }

    #[cfg(not(target_os = "windows"))]
    fn with_size_stub(width: u32, height: u32) -> Self {
        StaticBitmap {
            inner: Rc::new(RefCell::new(StaticBitmapInner {
                current_handle: 0,
                image_kind: StaticBitmapImageKind::None,
                rect: Rect::new(0, 0, width, height),
                visible: true,
                enabled: true,
            })),
        }
    }

    /// Apply a [`RawBitmap`] to the control. If the control already
    /// has an image, the previous one is released first.
    pub fn set_bitmap(&self, bmp: RawBitmap) {
        #[cfg(target_os = "windows")]
        {
            self.set_raw_bitmap(bmp.hbitmap);
            // Update the cached size from the bitmap's actual dimensions.
            let mut inner = self.inner.borrow_mut();
            inner.rect.width = bmp.width.max(1);
            inner.rect.height = bmp.height.max(1);
        }
        #[cfg(not(target_os = "windows"))]
        {
            let _ = bmp;
        }
    }

    /// Apply a raw `HBITMAP` to the control. If a previous bitmap was
    /// owned, it is released before the new one is set.
    #[cfg(target_os = "windows")]
    fn set_raw_bitmap(&self, hbitmap: HBITMAP) {
        if hbitmap.is_null() {
            return;
        }
        // Release any previous image of any kind.
        self.release_current();

        // Clone the bitmap so we can both display it and own a copy
        // (the system makes its own copy too, but we want to be
        // explicit about lifetime).
        let hbitmap_copy = clone_bitmap(hbitmap);
        let hwnd = self.inner.borrow().hwnd;
        // SAFETY: FFI call to SendMessageW; `hwnd` is a live static-bitmap control and `STM_SETIMAGE` / `IMAGE_BITMAP` / `hbitmap_copy` are valid for it.
        let _ = unsafe {
            SendMessageW(
                hwnd,
                STM_SETIMAGE,
                IMAGE_BITMAP,
                hbitmap_copy as isize as isize,
            )
        };
        self.inner.borrow_mut().current_handle = hbitmap_copy as isize;
        self.inner.borrow_mut().image_kind = StaticBitmapImageKind::Bitmap;
    }

    /// Apply a raw `HICON` to the control. If a previous image was
    /// owned, it is released before the new one is set.
    #[cfg(target_os = "windows")]
    fn set_icon(&self, hicon: HICON) {
        if hicon.is_null() {
            return;
        }
        self.release_current();

        let hicon_copy = clone_icon(hicon);
        let hwnd = self.inner.borrow().hwnd;
        // SAFETY: FFI call to SendMessageW; `hwnd` is a live static-bitmap control and `STM_SETIMAGE` / `IMAGE_ICON` / `hicon_copy` are valid for it.
        let _ = unsafe {
            SendMessageW(
                hwnd,
                STM_SETIMAGE,
                IMAGE_ICON,
                hicon_copy as isize as isize,
            )
        };
        self.inner.borrow_mut().current_handle = hicon_copy as isize;
        self.inner.borrow_mut().image_kind = StaticBitmapImageKind::Icon;
    }

    /// Clear the displayed image. Safe to call multiple times.
    pub fn clear(&self) {
        self.release_current();
        #[cfg(target_os = "windows")]
        {
            let hwnd = self.inner.borrow().hwnd;
            // SAFETY: FFI call to SendMessageW; `hwnd` is a live static-bitmap control and the message is a well-defined "no image" request.
            let _ = unsafe { SendMessageW(hwnd, STM_SETIMAGE, IMAGE_BITMAP, 0) };
        }
    }

    /// Release the currently-owned image, if any, by calling the
    /// matching Win32 destructor (`DeleteObject` for bitmaps, no-op for
    /// icons that we did not create — see `clone_icon`).
    #[cfg(target_os = "windows")]
    fn release_current(&self) {
        let (handle, kind) = {
            let inner = self.inner.borrow();
            (inner.current_handle, inner.image_kind)
        };
        if handle == 0 {
            return;
        }
        if kind == StaticBitmapImageKind::Bitmap {
            // SAFETY: FFI call to DeleteObject on a GDI handle we own.
            unsafe {
                DeleteObject(handle as HBITMAP);
            }
        }
        // Icons: we cloned via CopyIcon, so we must release with
        // DestroyIcon.
        else if kind == StaticBitmapImageKind::Icon {
            // SAFETY: FFI call to DestroyIcon on a cursor / icon handle we own (cloned via CopyIcon).
            unsafe {
                windows_sys::Win32::UI::WindowsAndMessaging::DestroyIcon(handle as HICON);
            }
        }
        let mut inner = self.inner.borrow_mut();
        inner.current_handle = 0;
        inner.image_kind = StaticBitmapImageKind::None;
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

// ── Icon / Bitmap clone helpers ────────────────────────────────────────

/// Make a copy of an `HBITMAP` we can own. We use `GetObjectW` to read
/// the dimensions, then re-render into a new DIB section via
/// `CreateDIBSection` + `GetDIBits`.
#[cfg(target_os = "windows")]
fn clone_bitmap(src: HBITMAP) -> HBITMAP {
    use windows_sys::Win32::Graphics::Gdi::{
        CreateCompatibleDC, GetDIBits, SelectObject, BITMAPINFO, BITMAPINFOHEADER,
        BI_RGB, DIB_RGB_COLORS, GetDC,
    };

    if src.is_null() {
        return std::ptr::null_mut();
    }

    // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
    unsafe {
        let mut info: BITMAP = std::mem::zeroed();
        let bytes = std::mem::size_of::<BITMAP>() as i32;
        let ok = GetObjectW(
            src as _,
            bytes,
            &mut info as *mut _ as *mut _,
        );
        if ok <= 0 {
            return std::ptr::null_mut();
        }

        let w = info.bmWidth;
        let h = info.bmHeight.abs();
        if w <= 0 || h <= 0 {
            return std::ptr::null_mut();
        }

        let screen_dc = GetDC(std::ptr::null_mut());
        if screen_dc.is_null() {
            return std::ptr::null_mut();
        }
        let mem_dc = CreateCompatibleDC(screen_dc);
        if mem_dc.is_null() {
            // Release the screen DC we acquired; the
            // compatible DC was never created.
            windows_sys::Win32::Graphics::Gdi::ReleaseDC(
                std::ptr::null_mut(),
                screen_dc,
            );
            return std::ptr::null_mut();
        }
        let old = SelectObject(mem_dc, src as _);

        let mut bmi: BITMAPINFO = std::mem::zeroed();
        bmi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
        bmi.bmiHeader.biWidth = w;
        bmi.bmiHeader.biHeight = h;
        bmi.bmiHeader.biPlanes = 1;
        bmi.bmiHeader.biBitCount = 32;
        bmi.bmiHeader.biCompression = BI_RGB;

        // Allocate a fresh DIB section to receive the bits.
        let mut bits_ptr: *mut std::ffi::c_void = std::ptr::null_mut();
        let new_bmp = windows_sys::Win32::Graphics::Gdi::CreateDIBSection(
            mem_dc,
            &bmi,
            DIB_RGB_COLORS,
            &mut bits_ptr,
            0 as _,
            0,
        );
        if new_bmp.is_null() || bits_ptr.is_null() {
            SelectObject(mem_dc, old);
            windows_sys::Win32::Graphics::Gdi::DeleteDC(mem_dc);
            windows_sys::Win32::Graphics::Gdi::ReleaseDC(std::ptr::null_mut(), screen_dc);
            return std::ptr::null_mut();
        }

        // Copy bits from src into the new DIB.
        let mut bmi2: BITMAPINFO = std::mem::zeroed();
        bmi2.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
        bmi2.bmiHeader.biWidth = w;
        bmi2.bmiHeader.biHeight = h;
        bmi2.bmiHeader.biPlanes = 1;
        bmi2.bmiHeader.biBitCount = 32;
        bmi2.bmiHeader.biCompression = BI_RGB;
        // Widening cast to `usize` *first* to avoid `u32`
        // overflow for large dimensions. See the matching
        // comment in `src/icon.rs`.
        let byte_count = (w as usize) * (h as usize) * 4;
        let dest = std::slice::from_raw_parts_mut(bits_ptr as *mut u8, byte_count);
        let got = GetDIBits(
            mem_dc,
            src,
            0,
            h as u32,
            dest.as_mut_ptr() as *mut _,
            &mut bmi2,
            DIB_RGB_COLORS,
        );

        SelectObject(mem_dc, old);
        windows_sys::Win32::Graphics::Gdi::DeleteDC(mem_dc);
        windows_sys::Win32::Graphics::Gdi::ReleaseDC(std::ptr::null_mut(), screen_dc);

        if got == 0 {
            DeleteObject(new_bmp);
            return std::ptr::null_mut();
        }
        new_bmp
    }
}

/// Make a copy of an `HICON` we can own (so we can `DestroyIcon` it
/// when the StaticBitmap is dropped without affecting the caller's
/// handle).
#[cfg(target_os = "windows")]
fn clone_icon(src: HICON) -> HICON {
    if src.is_null() {
        return std::ptr::null_mut();
    }
    // SAFETY: FFI call to CopyIcon; `src` is a live HICON.
    unsafe { windows_sys::Win32::UI::WindowsAndMessaging::CopyIcon(src) }
}

#[cfg(target_os = "windows")]
impl Window for StaticBitmap {
    fn hwnd(&self) -> HWND {
        self.hwnd()
    }
}

impl Widget for StaticBitmapInner {
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

#[cfg(target_os = "windows")]
impl Drop for StaticBitmapInner {
    fn drop(&mut self) {
        if self.current_handle != 0 {
            if self.image_kind == StaticBitmapImageKind::Bitmap {
                // SAFETY: FFI call to DeleteObject on a GDI handle we own.
                unsafe {
                    DeleteObject(self.current_handle as HBITMAP);
                }
            } else if self.image_kind == StaticBitmapImageKind::Icon {
                // SAFETY: FFI call to DestroyIcon on a cursor / icon handle we own (cloned via CopyIcon).
                unsafe {
                    windows_sys::Win32::UI::WindowsAndMessaging::DestroyIcon(
                        self.current_handle as HICON,
                    );
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_dimensions_are_positive() {
        #[allow(clippy::assertions_on_constants)]
        {
            assert!(StaticBitmap::DEFAULT_W > 0);
            assert!(StaticBitmap::DEFAULT_H > 0);
        }
    }

    #[test]
    fn image_kind_variants_compare_distinctly() {
        assert_ne!(StaticBitmapImageKind::None, StaticBitmapImageKind::Bitmap);
        assert_ne!(StaticBitmapImageKind::None, StaticBitmapImageKind::Icon);
        assert_ne!(StaticBitmapImageKind::Bitmap, StaticBitmapImageKind::Icon);
    }
}
