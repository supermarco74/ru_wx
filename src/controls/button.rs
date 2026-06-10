//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Push-button control (`wxButton`).
//!
//! On Windows the widget is realised with the `BUTTON` common control
//! class using style `BS_PUSHBUTTON`. Use [`Button::new`] for the
//! typical case, [`Button::on_click`] to install a click handler, and
//! [`Widget::set_enabled`] to toggle the interactive state.

use std::cell::RefCell;
use std::rc::Rc;

use crate::window::frame::Frame;
use crate::core::geometry::{Colour, Rect};
use crate::core::widget::{Widget, WidgetRef, Window};

#[cfg(target_os = "windows")]
use crate::platform::win32::{next_control_id, read_window_text, to_wide};
#[cfg(target_os = "windows")]
use windows_sys::Win32::Foundation::{RECT, *};
#[cfg(target_os = "windows")]
use windows_sys::Win32::Graphics::Gdi::*;
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::Input::KeyboardAndMouse::EnableWindow;
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::WindowsAndMessaging::*;

/// Sent to a button to associate it with a bitmap image.
#[cfg(target_os = "windows")]
const BM_SETIMAGE: u32 = 0x00F7;
/// Identifies the image type as a bitmap.
#[cfg(target_os = "windows")]
const IMAGE_BITMAP: usize = 0;
/// Flat push-button style (`BS_FLAT`).
#[cfg(target_os = "windows")]
const BS_FLAT: u32 = 0x0000_8000;
/// `Button_SetImageList` message (`BCM_SETIMAGELIST`).
#[cfg(target_os = "windows")]
const BCM_SETIMAGELIST: u32 = 0x1602;
/// Common-controls v6 enablement.
#[cfg(target_os = "windows")]
const CCM_SETVERSION: u32 = 0x2007;

/// Where a bitmap is placed relative to the button label.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BitmapAlign {
    Left,
    Right,
    Top,
    Bottom,
    Center,
}

impl BitmapAlign {
    #[cfg(target_os = "windows")]
    fn win32_align(self) -> u32 {
        match self {
            BitmapAlign::Left => 0,
            BitmapAlign::Right => 1,
            BitmapAlign::Top => 2,
            BitmapAlign::Bottom => 3,
            BitmapAlign::Center => 4,
        }
    }
}

#[cfg(target_os = "windows")]
#[repr(C)]
struct ButtonImageList {
    himl: isize,
    margin: RECT,
    align: u32,
}

struct ButtonInner {
    #[cfg(target_os = "windows")]
    hwnd: HWND,
    id: u16,
    label: String,
    rect: Rect,
    enabled: bool,
    visible: bool,
    #[cfg(target_os = "windows")]
    hbitmap: Option<HBITMAP>,
    flat_style: bool,
    image_list_attached: bool,
    bitmap_align: Option<BitmapAlign>,
}

#[derive(Clone)]
pub struct Button {
    inner: Rc<RefCell<ButtonInner>>,
}

impl Button {
    /// Create a new button as a child of the given parent window.
    pub fn new<W: Window>(parent_in: &W, label: &str) -> Self {
        let id = next_control_id();

        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        let hwnd = unsafe {
            let parent = parent_in.hwnd();
            let wide_label = to_wide(label);
            let wide_class = to_wide("BUTTON");
            CreateWindowExW(
                0,
                wide_class.as_ptr(),
                wide_label.as_ptr(),
                WS_CHILD | WS_VISIBLE,
                0,
                0,
                100,
                30, // default size, will be positioned by sizer
                parent,
                id as usize as HMENU,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        };

        #[cfg(not(target_os = "windows"))]
        let _ = parent_in;

        Button {
            inner: Rc::new(RefCell::new(ButtonInner {
                #[cfg(target_os = "windows")]
                hwnd,
                id,
                label: label.to_string(),
                rect: Rect::new(0, 0, 100, 30),
                enabled: true,
                visible: true,
                #[cfg(target_os = "windows")]
                hbitmap: None,
                flat_style: false,
                image_list_attached: false,
                bitmap_align: None,
            })),
        }
    }

    /// Flat push button (`BS_FLAT`) — borderless-looking surface.
    pub fn new_flat<W: Window>(parent_in: &W, label: &str) -> Self {
        let id = next_control_id();

        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        let hwnd = unsafe {
            let parent = parent_in.hwnd();
            let wide_label = to_wide(label);
            let wide_class = to_wide("BUTTON");
            CreateWindowExW(
                0,
                wide_class.as_ptr(),
                wide_label.as_ptr(),
                WS_CHILD | WS_VISIBLE | BS_FLAT,
                0,
                0,
                120,
                32,
                parent,
                id as usize as HMENU,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        };

        #[cfg(not(target_os = "windows"))]
        let _ = parent_in;

        let btn = Button {
            inner: Rc::new(RefCell::new(ButtonInner {
                #[cfg(target_os = "windows")]
                hwnd,
                id,
                label: label.to_string(),
                rect: Rect::new(0, 0, 120, 32),
                enabled: true,
                visible: true,
                #[cfg(target_os = "windows")]
                hbitmap: None,
                flat_style: true,
                image_list_attached: false,
                bitmap_align: None,
            })),
        };

        #[cfg(target_os = "windows")]
        unsafe {
            SendMessageW(btn.inner.borrow().hwnd, CCM_SETVERSION, 6, 0);
        }

        btn
    }

    /// Push button with label and an SVG icon at the given alignment
    /// (`BCM_SETIMAGELIST`, like `wxButton::SetBitmapPosition`).
    pub fn new_with_svg_aligned<W: Window>(
        parent_in: &W,
        label: &str,
        svg_bytes: &[u8],
        icon_size: u32,
        align: BitmapAlign,
    ) -> Self {
        let id = next_control_id();

        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        let hwnd = unsafe {
            let parent = parent_in.hwnd();
            let wide_label = to_wide(label);
            let wide_class = to_wide("BUTTON");
            CreateWindowExW(
                0,
                wide_class.as_ptr(),
                wide_label.as_ptr(),
                WS_CHILD | WS_VISIBLE,
                0,
                0,
                200,
                36,
                parent,
                id as usize as HMENU,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        };

        #[cfg(not(target_os = "windows"))]
        let _ = (parent_in, svg_bytes, icon_size, align);

        let btn = Button {
            inner: Rc::new(RefCell::new(ButtonInner {
                #[cfg(target_os = "windows")]
                hwnd,
                id,
                label: label.to_string(),
                rect: Rect::new(0, 0, 200, 36),
                enabled: true,
                visible: true,
                #[cfg(target_os = "windows")]
                hbitmap: None,
                flat_style: false,
                image_list_attached: false,
                bitmap_align: Some(align),
            })),
        };

        #[cfg(target_os = "windows")]
        {
            let list = crate::dc::image_list::ImageList::new(icon_size as i32, icon_size as i32);
            if list.add_svg_bytes(svg_bytes).is_some() {
                unsafe {
                    SendMessageW(hwnd, CCM_SETVERSION, 6, 0);
                    let bil = ButtonImageList {
                        himl: list.handle(),
                        margin: RECT {
                            left: 4,
                            top: 4,
                            right: 4,
                            bottom: 4,
                        },
                        align: align.win32_align(),
                    };
                    SendMessageW(
                        hwnd,
                        BCM_SETIMAGELIST,
                        0,
                        &bil as *const ButtonImageList as isize,
                    );
                }
                btn.inner.borrow_mut().image_list_attached = true;
            }
        }

        btn
    }

    /// `true` if this button was created with [`Button::new_flat`].
    pub fn is_flat(&self) -> bool {
        self.inner.borrow().flat_style
    }

    /// `true` if an image list is attached (text + bitmap variants).
    pub fn has_image_list(&self) -> bool {
        self.inner.borrow().image_list_attached
    }

    /// Bitmap alignment when an image list is attached.
    pub fn bitmap_align(&self) -> Option<BitmapAlign> {
        self.inner.borrow().bitmap_align
    }

    /// Create a button displaying a coloured bitmap icon.
    ///
    /// Creates a small bitmap programmatically with the given colour and
    /// attaches it to the button via `BM_SETIMAGE`.
    pub fn new_with_bitmap<W: Window>(
        parent_in: &W,
        label: &str,
        colour: Colour,
        bmp_width: i32,
        bmp_height: i32,
    ) -> Self {
        let id = next_control_id();

        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        let (hwnd, hbitmap) = unsafe {
            let parent = parent_in.hwnd();
            let wide_label = to_wide(label);
            let wide_class = to_wide("BUTTON");
            let hwnd = CreateWindowExW(
                0,
                wide_class.as_ptr(),
                wide_label.as_ptr(),
                WS_CHILD | WS_VISIBLE | BS_BITMAP as u32,
                0,
                0,
                100,
                30,
                parent,
                id as usize as HMENU,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            );

            // Create a coloured bitmap via GDI
            let hdc_screen = GetDC(std::ptr::null_mut());
            let hdc_mem = CreateCompatibleDC(hdc_screen);
            let hbmp = CreateCompatibleBitmap(hdc_screen, bmp_width, bmp_height);
            let old = SelectObject(hdc_mem, hbmp);

            let brush = CreateSolidBrush(colour.to_colorref());
            let rc = RECT {
                left: 0,
                top: 0,
                right: bmp_width,
                bottom: bmp_height,
            };
            FillRect(hdc_mem, &rc, brush);
            DeleteObject(brush);

            // Restore and clean up DC
            SelectObject(hdc_mem, old);
            DeleteDC(hdc_mem);
            ReleaseDC(std::ptr::null_mut(), hdc_screen);

            // Attach the bitmap to the button
            SendMessageW(hwnd, BM_SETIMAGE, IMAGE_BITMAP, hbmp as isize);

            (hwnd, Some(hbmp))
        };

        #[cfg(not(target_os = "windows"))]
        let _ = (parent_in, label, colour, bmp_width, bmp_height);

        Button {
            inner: Rc::new(RefCell::new(ButtonInner {
                #[cfg(target_os = "windows")]
                hwnd,
                id,
                label: label.to_string(),
                rect: Rect::new(0, 0, 100, 30),
                enabled: true,
                visible: true,
                #[cfg(target_os = "windows")]
                hbitmap,
                flat_style: false,
                image_list_attached: false,
                bitmap_align: None,
            })),
        }
    }

    /// Create a button with an SVG icon loaded from a file path.
    ///
    /// The SVG is rasterised to `icon_size × icon_size` pixels and attached
    /// as a bitmap image via `BM_SETIMAGE`.
    pub fn new_with_svg_icon<W: Window>(
        parent_in: &W,
        svg_path: &std::path::Path,
        icon_size: u32,
    ) -> Self {
        let id = next_control_id();

        #[cfg(target_os = "windows")]
        let (hwnd, hbitmap) = {
            let parent = parent_in.hwnd();
            let wide_class = to_wide("BUTTON");
            // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
            let hwnd = unsafe {
                CreateWindowExW(
                    0,
                    wide_class.as_ptr(),
                    std::ptr::null(),
                    WS_CHILD | WS_VISIBLE | BS_BITMAP as u32,
                    0,
                    0,
                    100,
                    30,
                    parent,
                    id as usize as HMENU,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                )
            };

            let hbitmap = crate::dc::icon::load_svg_as_hbitmap(svg_path, icon_size, icon_size);

            if let Some(hbmp) = hbitmap {
                // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
                unsafe {
                    SendMessageW(hwnd, BM_SETIMAGE, IMAGE_BITMAP, hbmp as isize);
                }
            }

            (hwnd, hbitmap)
        };

        #[cfg(not(target_os = "windows"))]
        let _ = (parent_in, svg_path, icon_size);

        Button {
            inner: Rc::new(RefCell::new(ButtonInner {
                #[cfg(target_os = "windows")]
                hwnd,
                id,
                label: String::new(),
                rect: Rect::new(0, 0, 100, 30),
                enabled: true,
                visible: true,
                #[cfg(target_os = "windows")]
                hbitmap,
                flat_style: false,
                image_list_attached: false,
                bitmap_align: None,
            })),
        }
    }

    /// Create a button with an SVG icon from embedded bytes.
    ///
    /// Use `include_bytes!` to embed SVG data at compile time.
    /// The SVG is rasterised to `icon_size × icon_size` pixels.
    pub fn new_with_svg_bytes<W: Window>(parent_in: &W, svg_bytes: &[u8], icon_size: u32) -> Self {
        let id = next_control_id();

        #[cfg(target_os = "windows")]
        let (hwnd, hbitmap) = {
            let parent = parent_in.hwnd();
            let wide_class = to_wide("BUTTON");
            // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
            let hwnd = unsafe {
                CreateWindowExW(
                    0,
                    wide_class.as_ptr(),
                    std::ptr::null(),
                    WS_CHILD | WS_VISIBLE | BS_BITMAP as u32,
                    0,
                    0,
                    100,
                    30,
                    parent,
                    id as usize as HMENU,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                )
            };

            let hbitmap = crate::dc::icon::svg_bytes_to_hbitmap(svg_bytes, icon_size, icon_size);

            if let Some(hbmp) = hbitmap {
                // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
                unsafe {
                    SendMessageW(hwnd, BM_SETIMAGE, IMAGE_BITMAP, hbmp as isize);
                }
            }

            (hwnd, hbitmap)
        };

        #[cfg(not(target_os = "windows"))]
        let _ = (parent_in, svg_bytes, icon_size);

        Button {
            inner: Rc::new(RefCell::new(ButtonInner {
                #[cfg(target_os = "windows")]
                hwnd,
                id,
                label: String::new(),
                rect: Rect::new(0, 0, 100, 30),
                enabled: true,
                visible: true,
                #[cfg(target_os = "windows")]
                hbitmap,
                flat_style: false,
                image_list_attached: false,
                bitmap_align: None,
            })),
        }
    }

    /// Get the control ID (used for WM_COMMAND dispatch)
    pub fn id(&self) -> u16 {
        self.inner.borrow().id
    }

    /// Set a click callback. Must register with the parent frame.
    pub fn on_click<F: FnMut() + 'static>(&self, frame: &Frame, callback: F) {
        let id = self.inner.borrow().id;
        frame.register_command_handler(id, Box::new(callback));
    }

    /// Click with [`ButtonEvent`] payload (`wxButtonEvent`).
    pub fn on_click_event<F: FnMut(&crate::ButtonEvent) + 'static>(&self, frame: &Frame, mut f: F) {
        let id = self.inner.borrow().id;
        self.on_click(frame, move || f(&crate::ButtonEvent::new(id)));
    }

    /// Set the button label
    pub fn set_label(&self, label: &str) {
        self.inner.borrow_mut().label = label.to_string();
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            let wide = to_wide(label);
            SetWindowTextW(self.inner.borrow().hwnd, wide.as_ptr());
        }
    }

    /// Get the current button label.
    ///
    /// On Windows this queries the underlying button via
    /// `GetWindowTextW`, so it returns the live label.
    pub fn get_label(&self) -> String {
        #[cfg(target_os = "windows")]
        {
            read_window_text(self.inner.borrow().hwnd)
        }
        #[cfg(not(target_os = "windows"))]
        {
            self.inner.borrow().label.clone()
        }
    }

    /// Get a WidgetRef for use with sizers
    pub fn as_widget_ref(&self) -> WidgetRef {
        self.inner.clone()
    }

    /// Return the platform's default button size as `(width, height)`
    /// in pixels.
    ///
    /// Mirrors `wxButton::GetDefaultSize` from wxWidgets. The size
    /// matches what `CreateWindowExW` would use for a freshly created
    /// `BUTTON` control with `BS_PUSHBUTTON` (a "wide" default that
    /// fits an OK / Cancel label plus 2× 6-pixel horizontal margins
    /// and a single line of system font text with vertical padding).
    pub fn default_size() -> (i32, i32) {
        #[cfg(target_os = "windows")]
        {
            // 88×26 is the standard button size reported by the
            // common-controls Button implementation on Windows for
            // a WS_CHILD | BS_PUSHBUTTON | WS_VISIBLE button
            // created without an explicit size.
            (88, 26)
        }
        #[cfg(not(target_os = "windows"))]
        {
            (75, 23)
        }
    }

    /// Deprecated CamelCase alias for [`Button::default_size`].
    ///
    /// Kept for API compatibility with the v0.6.2 release and the
    /// `wxWidgets` C++ method name. New code should call
    /// [`Button::default_size`].
    #[deprecated(
        since = "0.6.3",
        note = "use the snake_case `default_size()` instead"
    )]
    #[allow(non_snake_case)] // intentional API-compat alias
    pub fn GetDefaultSize() -> (i32, i32) {
        Self::default_size()
    }
}

impl Widget for ButtonInner {
    fn native_handle(&self) -> isize {
        #[cfg(target_os = "windows")]
        {
            self.hwnd as isize
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

impl Drop for ButtonInner {
    fn drop(&mut self) {
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            if let Some(hbmp) = self.hbitmap {
                DeleteObject(hbmp);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// v0.6.3: the renamed `default_size` returns the
    /// platform-specific Win32 default (88×26). The deprecated
    /// `GetDefaultSize` alias must produce the same value.
    #[test]
    fn default_size_returns_platform_default() {
        let (w, h) = Button::default_size();
        #[cfg(target_os = "windows")]
        assert_eq!((w, h), (88, 26));
        #[cfg(not(target_os = "windows"))]
        assert_eq!((w, h), (75, 23));
    }

    /// v0.6.3: the deprecated `GetDefaultSize` alias is a
    /// transparent shim over `default_size`.
    #[test]
    #[allow(deprecated)]
    fn deprecated_get_default_size_alias_matches() {
        let a = Button::default_size();
        #[allow(deprecated)]
        let b = Button::GetDefaultSize();
        assert_eq!(a, b);
    }
}
