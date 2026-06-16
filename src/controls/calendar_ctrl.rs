//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Month calendar (`wxCalendarCtrl`) — Win32 `SysMonthCal32`.

use std::cell::RefCell;
use std::rc::Rc;

use crate::controls::date_picker_ctrl::Date;
use crate::core::geometry::Rect;
use crate::core::widget::{Widget, WidgetRef, Window};

use crate::platform::next_control_id;
#[cfg(target_os = "windows")]
use crate::platform::win32::to_wide;
#[cfg(target_os = "windows")]
use windows_sys::Win32::Foundation::*;
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::WindowsAndMessaging::*;

#[cfg(target_os = "windows")]
const MCM_GETCURSEL: u32 = 0x1001;
#[cfg(target_os = "windows")]
const MCM_SETCURSEL: u32 = 0x1002;

struct CalendarCtrlInner {
    #[cfg(target_os = "windows")]
    hwnd: HWND,
    id: u16,
    rect: Rect,
    visible: bool,
}

#[derive(Clone)]
pub struct CalendarCtrl {
    inner: Rc<RefCell<CalendarCtrlInner>>,
}

impl CalendarCtrl {
    pub fn new<W: Window>(parent: &W) -> Self {
        let id = next_control_id();

        #[cfg(target_os = "windows")]
        let hwnd = unsafe {
            let class = to_wide("SysMonthCal32");
            CreateWindowExW(
                0,
                class.as_ptr(),
                std::ptr::null(),
                WS_CHILD | WS_VISIBLE,
                0,
                0,
                220,
                180,
                parent.hwnd(),
                id as usize as HMENU,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        };

        #[cfg(not(target_os = "windows"))]
        let _ = parent;

        Self {
            inner: Rc::new(RefCell::new(CalendarCtrlInner {
                #[cfg(target_os = "windows")]
                hwnd,
                id,
                rect: Rect::new(0, 0, 220, 180),
                visible: true,
            })),
        }
    }

    pub fn set_date(&self, date: Date) {
        #[cfg(target_os = "windows")]
        unsafe {
            let hwnd = self.inner.borrow().hwnd;
            let mut st = windows_sys::Win32::Foundation::SYSTEMTIME {
                wYear: date.year as u16,
                wMonth: date.month as u16,
                wDay: date.day as u16,
                wDayOfWeek: 0,
                wHour: 0,
                wMinute: 0,
                wSecond: 0,
                wMilliseconds: 0,
            };
            SendMessageW(hwnd, MCM_SETCURSEL, 0, &mut st as *mut _ as isize);
        }
        #[cfg(not(target_os = "windows"))]
        let _ = date;
    }

    pub fn get_date(&self) -> Option<Date> {
        #[cfg(target_os = "windows")]
        unsafe {
            let hwnd = self.inner.borrow().hwnd;
            let mut st: windows_sys::Win32::Foundation::SYSTEMTIME = std::mem::zeroed();
            if SendMessageW(hwnd, MCM_GETCURSEL, 0, &mut st as *mut _ as isize) == 0 {
                return None;
            }
            Some(Date::new(
                st.wYear as i32,
                st.wMonth as u32,
                st.wDay as u32,
            ))
        }
        #[cfg(not(target_os = "windows"))]
        {
            None
        }
    }

    pub fn as_widget_ref(&self) -> WidgetRef {
        self.inner.clone()
    }
}

impl Widget for CalendarCtrlInner {
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
        unsafe {
            ShowWindow(self.hwnd, if visible { SW_SHOW } else { SW_HIDE });
        }
    }

    fn is_enabled(&self) -> bool {
        true
    }

    fn set_enabled(&mut self, _enabled: bool) {}
}
