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

use crate::core::geometry::{Colour, Rect};
use crate::core::widget::{Widget, WidgetRef, Window};
use crate::window::frame::Frame;

use crate::platform::next_control_id;
#[cfg(target_os = "windows")]
use crate::platform::win32::{read_window_text, to_wide};
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
/// `BCM_GETIDEALSIZE` — dimensione ideale del bottone secondo il tema.
#[cfg(target_os = "windows")]
const BCM_GETIDEALSIZE: u32 = 0x1601;

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

#[cfg(target_os = "macos")]
fn macos_button<W: Window>(
    parent_in: &W,
    label: &str,
    id: u16,
    width: u32,
    height: u32,
) -> (isize, crate::platform::NativeHandle) {
    let parent = crate::platform::window_handle(parent_in);
    let handle =
        crate::platform::appkit::create_button(parent, label, id, Rect::new(0, 0, width, height));
    (handle, parent)
}

#[cfg(target_os = "linux")]
fn gtk_button<W: Window>(
    parent_in: &W,
    label: &str,
    id: u16,
    width: u32,
    height: u32,
) -> (isize, crate::platform::NativeHandle) {
    let parent = crate::platform::window_handle(parent_in);
    let handle =
        crate::platform::gtk::create_button(parent, label, id, Rect::new(0, 0, width, height));
    (handle, parent)
}

struct ButtonInner {
    #[cfg(target_os = "windows")]
    hwnd: HWND,
    #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
    stub_handle: isize,
    #[cfg(target_os = "macos")]
    stub_handle: isize,
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    parent_handle: crate::platform::NativeHandle,
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
            let parent = crate::platform::window_handle_as_hwnd(parent_in);
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

        #[cfg(all(
            not(target_os = "windows"),
            not(target_os = "macos"),
            not(target_os = "linux")
        ))]
        let _ = parent_in;

        // Best-size reale del label: la default 100 px troncava i
        // bottoni con testo lungo (es. "Rename Home tab" del demo
        // AuiManager, il testo veniva tagliato su entrambi i lati).
        // La size ideale la chiede al controllo col tema attivo
        // (`BCM_GETIDEALSIZE`); in fallback si misura il testo col
        // pattern DT_CALCRECT di `static_text` e si aggiunge padding.
        #[cfg(target_os = "windows")]
        let btn_w: u32 = {
            let mut ideal = SIZE { cx: 100, cy: 30 };
            // SAFETY: Win32 FFI call, `ideal` è valid per la chiamata.
            unsafe {
                SendMessageW(hwnd, BCM_GETIDEALSIZE, 0, &mut ideal as *mut SIZE as isize);
            }
            let text_w = if label.is_empty() {
                0
            } else {
                crate::controls::static_text::measure_text_size(hwnd, label).0 + 24
            };
            text_w.max(ideal.cx.max(0) as u32).clamp(100, 600)
        };
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments.
        unsafe {
            MoveWindow(hwnd, 0, 0, btn_w as i32, 30, 1);
        }
        #[cfg(not(target_os = "windows"))]
        let btn_w: u32 = 100;

        #[cfg(target_os = "macos")]
        let (stub_handle, parent_handle) = macos_button(parent_in, label, id, 100, 30);
        #[cfg(target_os = "linux")]
        let (stub_handle, parent_handle) = gtk_button(parent_in, label, id, 100, 30);

        Button {
            inner: Rc::new(RefCell::new(ButtonInner {
                #[cfg(target_os = "windows")]
                hwnd,
                #[cfg(all(
                    not(target_os = "windows"),
                    not(target_os = "macos"),
                    not(target_os = "linux")
                ))]
                stub_handle: crate::platform::stub_backend::alloc_widget_handle(),
                #[cfg(any(target_os = "macos", target_os = "linux"))]
                stub_handle,
                #[cfg(any(target_os = "macos", target_os = "linux"))]
                parent_handle,
                id,
                label: label.to_string(),
                rect: Rect::new(0, 0, btn_w, 30),
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
            let parent = crate::platform::window_handle_as_hwnd(parent_in);
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

        #[cfg(all(
            not(target_os = "windows"),
            not(target_os = "macos"),
            not(target_os = "linux")
        ))]
        let _ = parent_in;
        #[cfg(target_os = "macos")]
        let (stub_handle, parent_handle) = macos_button(parent_in, label, id, 120, 32);
        #[cfg(target_os = "linux")]
        let (stub_handle, parent_handle) = gtk_button(parent_in, label, id, 120, 32);

        let btn = Button {
            inner: Rc::new(RefCell::new(ButtonInner {
                #[cfg(target_os = "windows")]
                hwnd,
                #[cfg(all(
                    not(target_os = "windows"),
                    not(target_os = "macos"),
                    not(target_os = "linux")
                ))]
                stub_handle: crate::platform::stub_backend::alloc_widget_handle(),
                #[cfg(any(target_os = "macos", target_os = "linux"))]
                stub_handle,
                #[cfg(any(target_os = "macos", target_os = "linux"))]
                parent_handle,
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
            let parent = crate::platform::window_handle_as_hwnd(parent_in);
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

        #[cfg(all(
            not(target_os = "windows"),
            not(target_os = "macos"),
            not(target_os = "linux")
        ))]
        let _ = (parent_in, svg_bytes, icon_size, align);
        #[cfg(target_os = "macos")]
        let (stub_handle, parent_handle) = macos_button(parent_in, label, id, 200, 36);
        #[cfg(target_os = "linux")]
        let (stub_handle, parent_handle) = gtk_button(parent_in, label, id, 200, 36);

        let btn = Button {
            inner: Rc::new(RefCell::new(ButtonInner {
                #[cfg(target_os = "windows")]
                hwnd,
                #[cfg(all(
                    not(target_os = "windows"),
                    not(target_os = "macos"),
                    not(target_os = "linux")
                ))]
                stub_handle: crate::platform::stub_backend::alloc_widget_handle(),
                #[cfg(any(target_os = "macos", target_os = "linux"))]
                stub_handle,
                #[cfg(any(target_os = "macos", target_os = "linux"))]
                parent_handle,
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

        #[cfg(target_os = "macos")]
        {
            let image_only = label.is_empty();
            if crate::platform::appkit::button_set_svg_image(
                stub_handle,
                svg_bytes,
                icon_size,
                image_only,
            ) {
                btn.inner.borrow_mut().image_list_attached = true;
            }
            crate::platform::appkit::button_rewire_click_target(stub_handle, parent_handle, id);
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
            let parent = crate::platform::window_handle_as_hwnd(parent_in);
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

        #[cfg(all(
            not(target_os = "windows"),
            not(target_os = "macos"),
            not(target_os = "linux")
        ))]
        let _ = (parent_in, label, colour, bmp_width, bmp_height);
        #[cfg(target_os = "macos")]
        let (stub_handle, parent_handle) = macos_button(parent_in, label, id, 100, 30);
        #[cfg(target_os = "linux")]
        let (stub_handle, parent_handle) = gtk_button(parent_in, label, id, 100, 30);

        let btn = Button {
            inner: Rc::new(RefCell::new(ButtonInner {
                #[cfg(target_os = "windows")]
                hwnd,
                #[cfg(all(
                    not(target_os = "windows"),
                    not(target_os = "macos"),
                    not(target_os = "linux")
                ))]
                stub_handle: crate::platform::stub_backend::alloc_widget_handle(),
                #[cfg(any(target_os = "macos", target_os = "linux"))]
                stub_handle,
                #[cfg(any(target_os = "macos", target_os = "linux"))]
                parent_handle,
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
        };

        #[cfg(target_os = "macos")]
        {
            crate::platform::appkit::button_set_colour_image(
                stub_handle,
                colour.r,
                colour.g,
                colour.b,
                colour.a,
                bmp_width.max(1) as u32,
                bmp_height.max(1) as u32,
                label.is_empty(),
            );
            btn.inner.borrow_mut().image_list_attached = true;
        }

        btn
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
            let parent = crate::platform::window_handle_as_hwnd(parent_in);
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

        #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
        let _ = (parent_in, svg_path, icon_size);
        #[cfg(target_os = "macos")]
        let (stub_handle, parent_handle) = macos_button(parent_in, "★", id, 100, 30);
        #[cfg(target_os = "linux")]
        let (stub_handle, parent_handle) = gtk_button(parent_in, "★", id, 100, 30);

        Button {
            inner: Rc::new(RefCell::new(ButtonInner {
                #[cfg(target_os = "windows")]
                hwnd,
                #[cfg(all(
                    not(target_os = "windows"),
                    not(target_os = "macos"),
                    not(target_os = "linux")
                ))]
                stub_handle: crate::platform::stub_backend::alloc_widget_handle(),
                #[cfg(any(target_os = "macos", target_os = "linux"))]
                stub_handle,
                #[cfg(any(target_os = "macos", target_os = "linux"))]
                parent_handle,
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
            let parent = crate::platform::window_handle_as_hwnd(parent_in);
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

        #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
        let _ = (parent_in, svg_bytes, icon_size);
        #[cfg(target_os = "macos")]
        let (stub_handle, parent_handle) = macos_button(parent_in, "★", id, 100, 30);
        #[cfg(target_os = "linux")]
        let (stub_handle, parent_handle) = gtk_button(parent_in, "★", id, 100, 30);

        Button {
            inner: Rc::new(RefCell::new(ButtonInner {
                #[cfg(target_os = "windows")]
                hwnd,
                #[cfg(all(
                    not(target_os = "windows"),
                    not(target_os = "macos"),
                    not(target_os = "linux")
                ))]
                stub_handle: crate::platform::stub_backend::alloc_widget_handle(),
                #[cfg(any(target_os = "macos", target_os = "linux"))]
                stub_handle,
                #[cfg(any(target_os = "macos", target_os = "linux"))]
                parent_handle,
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

    /// Click with [`ButtonEvent`](crate::ButtonEvent) payload (`wxButtonEvent`).
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
        #[cfg(target_os = "macos")]
        {
            let handle = self.inner.borrow().stub_handle;
            crate::platform::appkit::set_button_title(handle, label);
        }
        #[cfg(target_os = "linux")]
        {
            let handle = self.inner.borrow().stub_handle;
            crate::platform::gtk::set_button_title(handle, label);
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

    /// Alias getter sintetico per il testo del pulsante (pendant di
    /// `set_text`). Delega a [`Self::get_label`], unificando i getter
    /// come da `PIANO_DEBUG_INSPECTOR.md` §A2.
    pub fn text(&self) -> String {
        self.get_label()
    }

    /// Setter sintetico per il testo del pulsante (pendant di
    /// [`Self::text`]). Delega a [`Self::set_label`].
    pub fn set_text(&self, text: &str) {
        self.set_label(text);
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
    #[deprecated(since = "0.6.3", note = "use the snake_case `default_size()` instead")]
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
        #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
        {
            self.stub_handle
        }
        #[cfg(target_os = "macos")]
        {
            self.stub_handle
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
        self.enabled
    }

    fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            EnableWindow(self.hwnd, if enabled { 1 } else { 0 });
        }
        #[cfg(target_os = "macos")]
        {
            crate::platform::appkit::set_view_enabled(self.stub_handle, enabled);
        }
        #[cfg(target_os = "linux")]
        {
            crate::platform::gtk::set_view_enabled(self.stub_handle, enabled);
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

#[cfg(all(test, target_os = "windows"))]
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

    /// A2: `get_label`/`set_label` passano dal testo nativo del
    /// bottone (senza inspector).
    #[test]
    fn button_label_roundtrip_reads_native_text() {
        let frame = Frame::builder()
            .with_title("button-getter")
            .with_size(240, 140)
            .build();
        let btn = Button::new(&frame, "Salva");
        assert_eq!(btn.get_label(), "Salva");
        btn.set_label("Annulla");
        assert_eq!(btn.get_label(), "Annulla");
    }
}
