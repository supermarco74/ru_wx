//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Rich text editor (`wxRichTextCtrl`) — Win32 RichEdit with formatting.

use std::cell::RefCell;
use std::rc::Rc;

use crate::core::geometry::Colour;
use crate::core::widget::{Widget, WidgetRef, Window};
use crate::platform::next_control_id;
#[cfg(target_os = "windows")]
use crate::platform::win32::to_wide;
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::Input::KeyboardAndMouse::EnableWindow;

use crate::window::frame::Frame;

#[cfg(target_os = "windows")]
use windows_sys::Win32::Foundation::*;
#[cfg(target_os = "windows")]
use windows_sys::Win32::System::LibraryLoader::LoadLibraryW;
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::WindowsAndMessaging::*;

#[cfg(target_os = "windows")]
const ES_MULTILINE: u32 = 0x0004;
#[cfg(target_os = "windows")]
const ES_AUTOVSCROLL: u32 = 0x0040;
#[cfg(target_os = "windows")]
const ES_WANTRETURN: u32 = 0x1000;
#[cfg(target_os = "windows")]
const EM_SETCHARFORMAT: u32 = 0x0444;
#[cfg(target_os = "windows")]
const SCF_SELECTION: u32 = 0x0001;
#[cfg(target_os = "windows")]
const CFM_BOLD: u32 = 0x0000_0001;
#[cfg(target_os = "windows")]
const CFM_ITALIC: u32 = 0x0000_0002;
#[cfg(target_os = "windows")]
const CFM_COLOR: u32 = 0x4000_0000;
#[cfg(target_os = "windows")]
const CFE_BOLD: u32 = 0x0001;
#[cfg(target_os = "windows")]
const CFE_ITALIC: u32 = 0x0002;

#[cfg(target_os = "windows")]
#[repr(C)]
struct CharFormat2W {
    cb_size: u32,
    dw_mask: u32,
    dw_effects: u32,
    y_height: i32,
    y_offset: i32,
    cr_text_color: u32,
    b_char_set: u8,
    b_pitch_and_family: u8,
    sz_face_name: [u16; 32],
    w_weight: u16,
    s_spacing: i16,
    cr_back_color: u32,
    lcid: u32,
    dw_reserved: u32,
    s_style: i16,
    w_kerning: u16,
    b_underline_type: u8,
    b_animation: u8,
    b_rev_author: u8,
    b_reserved1: u8,
}

struct RichTextCtrlInner {
    #[cfg(target_os = "windows")]
    hwnd: HWND,
    #[cfg(target_os = "macos")]
    stub_handle: crate::platform::NativeHandle,
    #[cfg(target_os = "macos")]
    parent_handle: crate::platform::NativeHandle,
    id: u16,
}

#[derive(Clone)]
pub struct RichTextCtrl {
    inner: Rc<RefCell<RichTextCtrlInner>>,
}

impl RichTextCtrl {
    pub fn new<W: Window>(parent: &W) -> Self {
        let id = next_control_id();
        #[cfg(target_os = "windows")]
        let hwnd = {
            // SAFETY: load RichEdit module once per control creation.
            unsafe {
                let dll = to_wide("Msftedit.dll");
                LoadLibraryW(dll.as_ptr());
            }
            let wide_class = to_wide("RichEdit20W");
            // SAFETY: create multiline RichEdit child window.
            unsafe {
                CreateWindowExW(
                    0,
                    wide_class.as_ptr(),
                    std::ptr::null(),
                    WS_CHILD | WS_VISIBLE | ES_MULTILINE | ES_AUTOVSCROLL | ES_WANTRETURN,
                    0,
                    0,
                    200,
                    120,
                    crate::platform::window_handle_as_hwnd(parent),
                    id as usize as HMENU,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                )
            }
        };
        #[cfg(target_os = "macos")]
        let (stub_handle, parent_handle) = {
            let parent = crate::platform::window_handle(parent);
            let handle = crate::platform::appkit::create_rich_text_ctrl(
                parent,
                crate::core::geometry::Rect::new(0, 0, 200, 120),
            );
            (handle, parent)
        };

        #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
        let _ = (parent, id);

        Self {
            inner: Rc::new(RefCell::new(RichTextCtrlInner {
                #[cfg(target_os = "windows")]
                hwnd,
                #[cfg(target_os = "macos")]
                stub_handle,
                #[cfg(target_os = "macos")]
                parent_handle,
                id,
            })),
        }
    }

    pub fn set_value(&self, value: &str) {
        #[cfg(target_os = "windows")]
        {
            let hwnd = self.inner.borrow().hwnd;
            let wide = to_wide(value);
            // SAFETY: set entire RichEdit buffer.
            unsafe {
                SetWindowTextW(hwnd, wide.as_ptr());
            }
        }
        #[cfg(target_os = "macos")]
        {
            crate::platform::appkit::rich_text_set_value(self.inner.borrow().stub_handle, value);
        }
        #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
        let _ = value;
    }

    pub fn value(&self) -> String {
        #[cfg(target_os = "windows")]
        {
            let hwnd = self.inner.borrow().hwnd;
            let len = unsafe { GetWindowTextLengthW(hwnd) } as usize;
            let mut buf = vec![0u16; len + 1];
            unsafe {
                GetWindowTextW(hwnd, buf.as_mut_ptr(), buf.len() as i32);
            }
            let end = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
            String::from_utf16_lossy(&buf[..end])
        }
        #[cfg(target_os = "macos")]
        {
            crate::platform::appkit::rich_text_get_value(self.inner.borrow().stub_handle)
        }
        #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
        {
            String::new()
        }
    }

    #[cfg(target_os = "windows")]
    pub fn set_bold(&self, bold: bool) {
        self.apply_char_format(CFM_BOLD, if bold { CFE_BOLD } else { 0 }, None);
    }

    #[cfg(target_os = "windows")]
    pub fn set_italic(&self, italic: bool) {
        self.apply_char_format(CFM_ITALIC, if italic { CFE_ITALIC } else { 0 }, None);
    }

    #[cfg(target_os = "windows")]
    pub fn set_text_colour(&self, colour: Colour) {
        self.apply_char_format(CFM_COLOR, 0, Some(colour.to_colorref()));
    }

    #[cfg(target_os = "macos")]
    pub fn set_bold(&self, bold: bool) {
        crate::platform::appkit::rich_text_set_bold(self.inner.borrow().stub_handle, bold);
    }

    #[cfg(target_os = "macos")]
    pub fn set_italic(&self, italic: bool) {
        crate::platform::appkit::rich_text_set_italic(self.inner.borrow().stub_handle, italic);
    }

    #[cfg(target_os = "macos")]
    pub fn set_text_colour(&self, colour: Colour) {
        crate::platform::appkit::rich_text_set_text_colour(self.inner.borrow().stub_handle, colour);
    }

    #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
    pub fn set_bold(&self, _bold: bool) {}

    #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
    pub fn set_italic(&self, _italic: bool) {}

    #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
    pub fn set_text_colour(&self, _colour: Colour) {}

    #[cfg(target_os = "macos")]
    pub fn set_underline(&self, underline: bool) {
        crate::platform::appkit::rich_text_set_underline(
            self.inner.borrow().stub_handle,
            underline,
        );
    }

    #[cfg(target_os = "windows")]
    pub fn set_underline(&self, underline: bool) {
        const CFM_UNDERLINE: u32 = 0x0040_0000;
        const CFE_UNDERLINE: u32 = 0x0004;
        self.apply_char_format(
            CFM_UNDERLINE,
            if underline { CFE_UNDERLINE } else { 0 },
            None,
        );
    }

    #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
    pub fn set_underline(&self, _underline: bool) {}

    #[cfg(target_os = "macos")]
    pub fn set_font_size(&self, size: f64) {
        crate::platform::appkit::rich_text_set_font_size(self.inner.borrow().stub_handle, size);
    }

    #[cfg(target_os = "windows")]
    pub fn set_font_size(&self, size: f64) {
        const CFM_SIZE: u32 = 0x8000_0000;
        let hwnd = self.inner.borrow().hwnd;
        let mut cf = CharFormat2W {
            cb_size: std::mem::size_of::<CharFormat2W>() as u32,
            dw_mask: CFM_SIZE,
            dw_effects: 0,
            y_height: (size * 20.0) as i32,
            y_offset: 0,
            cr_text_color: 0,
            b_char_set: 0,
            b_pitch_and_family: 0,
            sz_face_name: [0; 32],
            w_weight: 0,
            s_spacing: 0,
            cr_back_color: 0,
            lcid: 0,
            dw_reserved: 0,
            s_style: 0,
            w_kerning: 0,
            b_underline_type: 0,
            b_animation: 0,
            b_rev_author: 0,
            b_reserved1: 0,
        };
        unsafe {
            SendMessageW(
                hwnd,
                EM_SETCHARFORMAT,
                SCF_SELECTION as usize,
                &mut cf as *mut _ as LPARAM,
            );
        }
    }

    #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
    pub fn set_font_size(&self, _size: f64) {}

    #[cfg(target_os = "macos")]
    pub fn set_strikethrough(&self, strike: bool) {
        crate::platform::appkit::rich_text_set_strikethrough(
            self.inner.borrow().stub_handle,
            strike,
        );
    }

    #[cfg(target_os = "windows")]
    pub fn set_strikethrough(&self, strike: bool) {
        const CFM_STRIKEOUT: u32 = 0x0080_0000;
        const CFE_STRIKEOUT: u32 = 0x0008;
        self.apply_char_format(CFM_STRIKEOUT, if strike { CFE_STRIKEOUT } else { 0 }, None);
    }

    #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
    pub fn set_strikethrough(&self, _strike: bool) {}

    #[cfg(target_os = "macos")]
    pub fn set_alignment(&self, align: u8) {
        crate::platform::appkit::rich_text_set_alignment(self.inner.borrow().stub_handle, align);
    }

    #[cfg(target_os = "windows")]
    pub fn set_alignment(&self, align: u8) {
        const EM_SETPARAFORMAT: u32 = 0x0479;
        const PFM_ALIGNMENT: u32 = 0x0008;
        const PFA_LEFT: u16 = 1;
        const PFA_CENTER: u16 = 2;
        const PFA_RIGHT: u16 = 3;
        #[repr(C)]
        struct ParaFormat {
            cb_size: u32,
            w_reserved: u16,
            s_height: i16,
            s_indent: i16,
            dx_right_indent: i16,
            dx_start_indent: i16,
            w_numbering: u16,
            w_reserved2: u16,
            dx_offset: i16,
            w_alignment: u16,
            sz_tab: [i16; 32],
        }
        let alignment = match align {
            0 => PFA_LEFT,
            1 => PFA_CENTER,
            _ => PFA_RIGHT,
        };
        let mut pf = ParaFormat {
            cb_size: std::mem::size_of::<ParaFormat>() as u32,
            w_reserved: 0,
            s_height: 0,
            s_indent: 0,
            dx_right_indent: 0,
            dx_start_indent: 0,
            w_numbering: 0,
            w_reserved2: 0,
            dx_offset: 0,
            w_alignment: alignment,
            sz_tab: [0; 32],
        };
        let hwnd = self.inner.borrow().hwnd;
        // SAFETY: EM_SETPARAFORMAT on a live RichEdit HWND.
        unsafe {
            use windows_sys::Win32::UI::WindowsAndMessaging::SendMessageW;
            SendMessageW(hwnd, EM_SETPARAFORMAT, 0, &mut pf as *mut _ as isize);
        }
    }

    #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
    pub fn set_alignment(&self, _align: u8) {}

    #[cfg(target_os = "macos")]
    pub fn set_link(&self, url: &str) {
        crate::platform::appkit::rich_text_set_link(self.inner.borrow().stub_handle, url);
    }

    #[cfg(target_os = "windows")]
    pub fn set_link(&self, _url: &str) {
        const CFM_LINK: u32 = 0x0000_0020;
        const CFE_LINK: u32 = 0x0020;
        self.apply_char_format(CFM_LINK, CFE_LINK, None);
    }

    #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
    pub fn set_link(&self, _url: &str) {}

    #[cfg(target_os = "macos")]
    pub fn can_undo(&self) -> bool {
        crate::platform::appkit::rich_text_can_undo(self.inner.borrow().stub_handle)
    }

    #[cfg(target_os = "macos")]
    pub fn can_redo(&self) -> bool {
        crate::platform::appkit::rich_text_can_redo(self.inner.borrow().stub_handle)
    }

    #[cfg(target_os = "macos")]
    pub fn undo(&self) {
        crate::platform::appkit::rich_text_undo(self.inner.borrow().stub_handle);
    }

    #[cfg(target_os = "macos")]
    pub fn redo(&self) {
        crate::platform::appkit::rich_text_redo(self.inner.borrow().stub_handle);
    }

    #[cfg(target_os = "macos")]
    pub fn set_html(&self, html: &str) -> bool {
        crate::platform::appkit::rich_text_set_html(self.inner.borrow().stub_handle, html)
    }

    #[cfg(target_os = "macos")]
    pub fn html(&self) -> String {
        crate::platform::appkit::rich_text_get_html(self.inner.borrow().stub_handle)
    }

    #[cfg(target_os = "macos")]
    pub fn apply_attr(&self, attr: &crate::adv::rich_text_attr::RichTextAttr) {
        let handle = self.inner.borrow().stub_handle;
        if attr.bold {
            self.set_bold(true);
        }
        if attr.italic {
            self.set_italic(true);
        }
        if attr.underline {
            self.set_underline(true);
        }
        if attr.font_size > 0 {
            self.set_font_size(attr.font_size as f64);
        }
        if let Some(colour) = attr.text_colour {
            self.set_text_colour(colour);
        }
        if let Some(bg) = attr.background_colour {
            crate::platform::appkit::rich_text_set_background_colour(handle, bg);
        }
    }

    fn apply_char_format(&self, mask: u32, effects: u32, colour: Option<u32>) {
        #[cfg(target_os = "windows")]
        {
            let hwnd = self.inner.borrow().hwnd;
            let mut cf = CharFormat2W {
                cb_size: std::mem::size_of::<CharFormat2W>() as u32,
                dw_mask: mask,
                dw_effects: effects,
                y_height: 0,
                y_offset: 0,
                cr_text_color: colour.unwrap_or(0),
                b_char_set: 0,
                b_pitch_and_family: 0,
                sz_face_name: [0; 32],
                w_weight: 0,
                s_spacing: 0,
                cr_back_color: 0,
                lcid: 0,
                dw_reserved: 0,
                s_style: 0,
                w_kerning: 0,
                b_underline_type: 0,
                b_animation: 0,
                b_rev_author: 0,
                b_reserved1: 0,
            };
            // SAFETY: EM_SETCHARFORMAT on a live RichEdit HWND.
            unsafe {
                SendMessageW(
                    hwnd,
                    EM_SETCHARFORMAT,
                    SCF_SELECTION as usize,
                    &mut cf as *mut _ as LPARAM,
                );
            }
        }
        #[cfg(not(target_os = "windows"))]
        let _ = (mask, effects, colour);
    }

    pub fn as_widget_ref(&self) -> WidgetRef {
        self.inner.clone()
    }
}

impl Widget for RichTextCtrlInner {
    fn native_handle(&self) -> isize {
        #[cfg(target_os = "windows")]
        {
            self.hwnd as isize
        }
        #[cfg(target_os = "macos")]
        {
            self.stub_handle as isize
        }
        #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
        {
            0
        }
    }

    fn set_position(&mut self, x: i32, y: i32) {
        #[cfg(target_os = "windows")]
        unsafe {
            MoveWindow(self.hwnd, x, y, 200, 120, 1);
        }
        #[cfg(target_os = "macos")]
        {
            let mut rect = self.rect();
            rect.x = x;
            rect.y = y;
            crate::platform::appkit::macos_widget_set_frame(
                self.parent_handle,
                self.stub_handle,
                rect,
            );
        }
        let _ = (x, y);
    }

    fn set_size(&mut self, w: u32, h: u32) {
        #[cfg(target_os = "windows")]
        unsafe {
            MoveWindow(self.hwnd, 0, 0, w as i32, h as i32, 1);
        }
        #[cfg(target_os = "macos")]
        {
            let mut rect = self.rect();
            rect.width = w;
            rect.height = h;
            crate::platform::appkit::macos_widget_set_frame(
                self.parent_handle,
                self.stub_handle,
                rect,
            );
        }
        let _ = (w, h);
    }

    fn rect(&self) -> crate::core::geometry::Rect {
        crate::core::geometry::Rect::new(0, 0, 200, 120)
    }

    fn is_visible(&self) -> bool {
        true
    }

    fn set_visible(&mut self, visible: bool) {
        #[cfg(target_os = "windows")]
        unsafe {
            ShowWindow(self.hwnd, if visible { SW_SHOW } else { SW_HIDE });
        }
        let _ = visible;
    }

    fn is_enabled(&self) -> bool {
        true
    }

    fn set_enabled(&mut self, enabled: bool) {
        #[cfg(target_os = "windows")]
        unsafe {
            EnableWindow(self.hwnd, if enabled { 1 } else { 0 });
        }
        #[cfg(not(target_os = "windows"))]
        let _ = enabled;
    }
}

#[cfg(any(target_os = "windows", target_os = "macos"))]
impl RichTextCtrl {
    pub fn id(&self) -> u16 {
        self.inner.borrow().id
    }

    pub fn on_change(&self, frame: &Frame, f: impl FnMut() + 'static) {
        frame.register_command_handler(self.id(), Box::new(f));
        #[cfg(target_os = "macos")]
        crate::platform::appkit::rich_text_enable_change_notify(
            self.inner.borrow().stub_handle,
            self.id(),
        );
    }
}
