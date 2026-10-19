//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Read-only label control (`wxStaticText`).
//!
//! On Windows the widget is a `STATIC` child with style `SS_LEFT`.
//! Use [`StaticText::new`] to create one and [`StaticText::set_label`]
//! / [`StaticText::get_label`] to update / read the displayed text.

use std::cell::RefCell;
use std::rc::Rc;

use crate::core::font::Font;
use crate::core::geometry::{Colour, Rect};
use crate::core::widget::{Widget, WidgetRef, Window};
#[cfg(target_os = "windows")]
use crate::platform::next_control_id;

#[cfg(target_os = "windows")]
use crate::platform::win32::{read_window_text, to_wide};
#[cfg(target_os = "windows")]
use windows_sys::Win32::Foundation::*;
#[cfg(target_os = "windows")]
use windows_sys::Win32::Graphics::Gdi::{
    DrawTextW, GetDC, GetStockObject, ReleaseDC, SelectObject, DEFAULT_GUI_FONT, DT_CALCRECT,
};
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::WindowsAndMessaging::*;

/// Misura il rettangolo minimo necessario a `text` con il font del
/// controllo (`DrawTextW` + `DT_CALCRECT`, stesso pattern di
/// `tip_provider`). Senza `DT_WORDBREAK` la larghezza risultante è
/// quella della riga più lunga (niente troncature nei sizer), l'altezza
/// copre tutte le righe. Ritorna `(200, 20)` (default del controllo)
/// se la misurazione non è disponibile.
#[cfg(target_os = "windows")]
pub(crate) fn measure_text_size(hwnd: HWND, text: &str) -> (u32, u32) {
    if text.is_empty() {
        return (200, 20);
    }
    // SAFETY: `GetDC`/`ReleaseDC` su un controllo vivo; il DC viene
    // rilasciato in ogni percorso e il vecchio font ripristinato.
    unsafe {
        let hdc = GetDC(hwnd);
        if hdc.is_null() {
            return (200, 20);
        }
        // Font del controllo (se impostato via `set_font`), altrimenti
        // il font GUI di sistema.
        let hfont = SendMessageW(hwnd, WM_GETFONT, 0, 0);
        let old_font = if hfont != 0 {
            SelectObject(hdc, hfont as _)
        } else {
            SelectObject(hdc, GetStockObject(DEFAULT_GUI_FONT))
        };
        let wide = to_wide(text);
        let mut rc: RECT = RECT {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        };
        DrawTextW(
            hdc,
            wide.as_ptr(),
            (wide.len() - 1) as i32,
            &mut rc,
            DT_CALCRECT,
        );
        SelectObject(hdc, old_font);
        ReleaseDC(hwnd, hdc);
        let w = (rc.right - rc.left).max(1) as u32;
        let h = (rc.bottom - rc.top).max(1) as u32;
        (w, h)
    }
}

struct StaticTextInner {
    #[cfg(target_os = "windows")]
    hwnd: HWND,
    #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
    stub_handle: isize,
    #[cfg(target_os = "macos")]
    stub_handle: isize,
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    parent_handle: crate::platform::NativeHandle,
    label: String,
    rect: Rect,
    visible: bool,
    enabled: bool,
}

#[derive(Clone)]
pub struct StaticText {
    inner: Rc<RefCell<StaticTextInner>>,
}

impl StaticText {
    /// Create a new static text label as a child of the given parent window.
    pub fn new<W: Window>(parent_in: &W, text: &str) -> Self {
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        let hwnd = unsafe {
            let parent = crate::platform::window_handle_as_hwnd(parent_in);
            let wide_text = to_wide(text);
            let wide_class = to_wide("STATIC");
            CreateWindowExW(
                0,
                wide_class.as_ptr(),
                wide_text.as_ptr(),
                WS_CHILD | WS_VISIBLE,
                0,
                0,
                200,
                20,
                parent,
                next_control_id() as usize as HMENU,
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
        let parent_handle = crate::platform::window_handle(parent_in);
        #[cfg(target_os = "macos")]
        let stub_handle = crate::platform::appkit::create_static_text(
            parent_handle,
            text,
            Rect::new(0, 0, 200, 20),
        );
        #[cfg(target_os = "linux")]
        let parent_handle = crate::platform::window_handle(parent_in);
        #[cfg(target_os = "linux")]
        let stub_handle =
            crate::platform::gtk::create_static_text(parent_handle, text, Rect::new(0, 0, 200, 20));

        // Best-size reale del testo: senza questa misura i sizer
        // vedono il default 200×20 e le label lunghe finiscono
        // troncate (la larghezza disponibile è quella del rect).
        #[cfg(target_os = "windows")]
        let (w, h) = measure_text_size(hwnd, text);
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments.
        unsafe {
            MoveWindow(hwnd, 0, 0, w as i32, h as i32, 1);
        }
        #[cfg(not(target_os = "windows"))]
        let (w, h) = (200u32, 20u32);

        StaticText {
            inner: Rc::new(RefCell::new(StaticTextInner {
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
                label: text.to_string(),
                rect: Rect::new(0, 0, w, h),
                visible: true,
                enabled: true,
            })),
        }
    }

    /// Set the text content
    pub fn set_label(&self, text: &str) {
        self.inner.borrow_mut().label = text.to_string();
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments.
        unsafe {
            let wide = to_wide(text);
            let (w, h) = {
                let inner = self.inner.borrow();
                SetWindowTextW(inner.hwnd, wide.as_ptr());
                measure_text_size(inner.hwnd, text)
            };
            // Aggiorna rect + geometria nativa: il sizer legge `rect()`
            // (`effective_size`), quindi senza questo passo la label
            // resterebbe al vecchio size dopo il change.
            let mut inner = self.inner.borrow_mut();
            inner.rect.width = w;
            inner.rect.height = h;
            MoveWindow(
                inner.hwnd,
                inner.rect.x,
                inner.rect.y,
                w as i32,
                h as i32,
                1,
            );
        }
        #[cfg(target_os = "macos")]
        {
            crate::platform::appkit::set_static_text(self.inner.borrow().stub_handle, text);
        }
        #[cfg(target_os = "linux")]
        {
            crate::platform::gtk::set_static_text(self.inner.borrow().stub_handle, text);
        }
    }

    /// Get the current text content of the control.
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

    /// Apply a custom font to the control.
    pub fn set_font(&self, font: &Font) {
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            SendMessageW(
                self.inner.borrow().hwnd,
                WM_SETFONT,
                font.hfont() as usize,
                1,
            );
        }
        #[cfg(target_os = "macos")]
        {
            let handle = self.inner.borrow().stub_handle;
            let ns_font = crate::platform::appkit::mac_font_from_desc(font.desc());
            crate::platform::appkit::apply_font_to_view(handle, &ns_font);
        }
        #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
        let _ = font;
    }

    /// Set the label text colour (macOS; no-op on other targets).
    pub fn set_foreground_colour(&self, colour: Colour) {
        #[cfg(target_os = "macos")]
        {
            crate::platform::appkit::set_view_text_color(self.inner.borrow().stub_handle, colour);
        }
        #[cfg(not(target_os = "macos"))]
        let _ = colour;
    }

    /// Limit visible lines (macOS). `0` = wrap with no line cap.
    pub fn set_max_lines(&self, max_lines: i32) {
        #[cfg(target_os = "macos")]
        {
            crate::platform::appkit::static_label_set_max_lines(
                self.inner.borrow().stub_handle,
                max_lines,
            );
        }
        #[cfg(not(target_os = "macos"))]
        let _ = max_lines;
    }

    /// Get a WidgetRef for use with sizers
    pub fn as_widget_ref(&self) -> WidgetRef {
        self.inner.clone()
    }
}

impl Widget for StaticTextInner {
    fn native_handle(&self) -> isize {
        #[cfg(target_os = "windows")]
        {
            self.hwnd as isize
        }
        #[cfg(not(target_os = "windows"))]
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
        {
            // StaticText does not support EnableWindow on Win32; store state only
        }
        #[cfg(target_os = "macos")]
        {
            crate::platform::appkit::set_view_enabled(self.stub_handle, enabled);
        }
        #[cfg(target_os = "linux")]
        {
            crate::platform::gtk::set_view_enabled(self.stub_handle, enabled);
        }
        #[cfg(all(
            not(target_os = "windows"),
            not(target_os = "macos"),
            not(target_os = "linux")
        ))]
        let _ = enabled;
    }
}
