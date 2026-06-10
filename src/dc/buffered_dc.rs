//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Double-buffered DC (`wxBufferedDC`).

use crate::dc::dc::MemoryDC;

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

    /// Mark the buffer as copied to the target (stub until real blit wiring).
    pub fn mark_blitted(&mut self) {
        self.blitted = true;
    }

    pub fn was_blitted(&self) -> bool {
        self.blitted
    }
}
