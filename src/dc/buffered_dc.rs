//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Double-buffered DC (`wxBufferedDC`).

use crate::dc::dc::{Dc, MemoryDC};

#[cfg(target_os = "windows")]
use windows_sys::Win32::Foundation::HWND;
#[cfg(target_os = "windows")]
use windows_sys::Win32::Graphics::Gdi::{BitBlt, SRCCOPY};

/// Off-screen buffer blitted to a target on drop (`wxBufferedDC`).
pub struct BufferedDC {
    memory: MemoryDC,
    width: i32,
    height: i32,
    blitted: bool,
}

impl BufferedDC {
    pub fn new(width: i32, height: i32) -> Self {
        Self {
            memory: MemoryDC::new(),
            width,
            height,
            blitted: false,
        }
    }

    pub fn memory_dc(&mut self) -> &mut MemoryDC {
        &mut self.memory
    }

    pub fn size(&self) -> (i32, i32) {
        (self.width, self.height)
    }

    /// Copy the off-screen buffer into `target` at `(dest_x, dest_y)`.
    #[cfg(target_os = "windows")]
    pub fn blit_to(&mut self, target: &mut dyn Dc, dest_x: i32, dest_y: i32) {
        let src = target.handle();
        if src == 0 {
            return;
        }
        let mem = self.memory.handle();
        if mem == 0 {
            return;
        }
        // SAFETY: both DC handles are live GDI contexts.
        unsafe {
            let _ = BitBlt(
                src as _,
                dest_x,
                dest_y,
                self.width,
                self.height,
                mem as _,
                0,
                0,
                SRCCOPY,
            );
        }
        self.blitted = true;
    }

    #[cfg(not(target_os = "windows"))]
    pub fn blit_to(&mut self, _target: &mut dyn Dc, _dest_x: i32, _dest_y: i32) {
        self.blitted = true;
    }

    /// Blit the buffer onto a window client area.
    ///
    /// # Safety
    /// `hwnd` must be a valid window handle for the duration of the call.
    #[cfg(target_os = "windows")]
    pub unsafe fn blit_to_window(&mut self, hwnd: HWND, dest_x: i32, dest_y: i32) {
        use windows_sys::Win32::Graphics::Gdi::{GetDC, ReleaseDC};
        let hdc = GetDC(hwnd);
        if hdc.is_null() {
            return;
        }
        let mem = self.memory.handle();
        if mem != 0 {
            let _ = BitBlt(
                hdc,
                dest_x,
                dest_y,
                self.width,
                self.height,
                mem as _,
                0,
                0,
                SRCCOPY,
            );
            self.blitted = true;
        }
        ReleaseDC(hwnd, hdc);
    }

    #[cfg(not(target_os = "windows"))]
    pub fn blit_to_window(&mut self, _hwnd: isize, _dest_x: i32, _dest_y: i32) {
        self.blitted = true;
    }

    /// Mark the buffer as copied to the target (legacy helper).
    pub fn mark_blitted(&mut self) {
        self.blitted = true;
    }

    pub fn was_blitted(&self) -> bool {
        self.blitted
    }
}
