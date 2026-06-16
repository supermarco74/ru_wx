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
                    parent.hwnd(),
                    id as usize as HMENU,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                )
            }
        };
        #[cfg(not(target_os = "windows"))]
        let _ = (parent, id);

        Self {
            inner: Rc::new(RefCell::new(RichTextCtrlInner {
                #[cfg(target_os = "windows")]
                hwnd,
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
        #[cfg(not(target_os = "windows"))]
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
        #[cfg(not(target_os = "windows"))]
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

    #[cfg(not(target_os = "windows"))]
    pub fn set_bold(&self, _bold: bool) {}

    #[cfg(not(target_os = "windows"))]
    pub fn set_italic(&self, _italic: bool) {}

    #[cfg(not(target_os = "windows"))]
    pub fn set_text_colour(&self, _colour: Colour) {}

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
        #[cfg(not(target_os = "windows"))]
        {
            0
        }
    }

    fn set_position(&mut self, x: i32, y: i32) {
        #[cfg(target_os = "windows")]
        unsafe {
            MoveWindow(self.hwnd, x, y, 200, 120, 1);
        }
        let _ = (x, y);
    }

    fn set_size(&mut self, w: u32, h: u32) {
        #[cfg(target_os = "windows")]
        unsafe {
            MoveWindow(self.hwnd, 0, 0, w as i32, h as i32, 1);
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

    fn set_enabled(&mut self, _enabled: bool) {}
}

#[cfg(target_os = "windows")]
impl RichTextCtrl {
    pub fn id(&self) -> u16 {
        self.inner.borrow().id
    }

    pub fn on_change(&self, frame: &Frame, f: impl FnMut() + 'static) {
        frame.register_command_handler(self.id(), Box::new(f));
    }
}
