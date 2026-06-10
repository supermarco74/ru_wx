//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Display / monitor information (`wxDisplay`).

use crate::core::geometry::Rect;

/// One screen or monitor (`wxDisplay`).
#[derive(Debug, Clone, Copy)]
pub struct Display {
    pub index: u32,
    pub rect: Rect,
    pub work_area: Rect,
    pub primary: bool,
}

impl Display {
    pub const fn new(index: u32, rect: Rect, work_area: Rect, primary: bool) -> Self {
        Self {
            index,
            rect,
            work_area,
            primary,
        }
    }

    /// Enumerate connected displays.
    pub fn enumerate() -> Vec<Display> {
        #[cfg(target_os = "windows")]
        {
            use windows_sys::Win32::UI::WindowsAndMessaging::{
                GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN, SM_CXVIRTUALSCREEN,
                SM_CYVIRTUALSCREEN, SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN,
            };

            let w = unsafe { GetSystemMetrics(SM_CXSCREEN) } as u32;
            let h = unsafe { GetSystemMetrics(SM_CYSCREEN) } as u32;
            let vw = unsafe { GetSystemMetrics(SM_CXVIRTUALSCREEN) } as u32;
            let vh = unsafe { GetSystemMetrics(SM_CYVIRTUALSCREEN) } as u32;
            let vx = unsafe { GetSystemMetrics(SM_XVIRTUALSCREEN) };
            let vy = unsafe { GetSystemMetrics(SM_YVIRTUALSCREEN) };
            let primary = Display::new(
                0,
                Rect::new(0, 0, w, h),
                Rect::new(0, 0, w, h.saturating_sub(40)),
                true,
            );
            if vw > w || vh > h {
                vec![
                    primary,
                    Display::new(
                        1,
                        Rect::new(vx + w as i32, vy, vw.saturating_sub(w), vh),
                        Rect::new(vx + w as i32, vy, vw.saturating_sub(w), vh.saturating_sub(40)),
                        false,
                    ),
                ]
            } else {
                vec![primary]
            }
        }
        #[cfg(not(target_os = "windows"))]
        {
            vec![Display::new(
                0,
                Rect::new(0, 0, 1920, 1080),
                Rect::new(0, 0, 1920, 1040),
                true,
            )]
        }
    }

    pub fn primary() -> Option<Display> {
        Self::enumerate().into_iter().find(|d| d.primary)
    }
}
