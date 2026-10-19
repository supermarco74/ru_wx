//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Double-buffered paint DC (`wxAutoBufferedPaintDC`).

use crate::dc::dc::MemoryDC;

/// Buffered paint context (`wxAutoBufferedPaintDC`).
pub struct AutoBufferedPaintDC {
    memory: MemoryDC,
    width: i32,
    height: i32,
}

impl AutoBufferedPaintDC {
    #[cfg(target_os = "windows")]
    pub fn new(width: i32, height: i32) -> Self {
        Self {
            memory: MemoryDC::new(),
            width,
            height,
        }
    }

    #[cfg(target_os = "macos")]
    pub fn new(width: i32, height: i32) -> Self {
        Self {
            memory: MemoryDC::new(),
            width,
            height,
        }
    }

    #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
    pub fn new(width: i32, height: i32) -> Self {
        Self {
            memory: MemoryDC::new(),
            width,
            height,
        }
    }

    pub fn memory_dc(&mut self) -> &mut MemoryDC {
        &mut self.memory
    }

    pub fn size(&self) -> (i32, i32) {
        (self.width, self.height)
    }
}
