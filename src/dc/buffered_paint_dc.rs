//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Buffered paint DC (`wxBufferedPaintDC`).

use crate::dc::dc::MemoryDC;

/// Paint-time double buffer (`wxBufferedPaintDC`).
pub struct BufferedPaintDC {
    memory: MemoryDC,
    width: i32,
    height: i32,
}

impl BufferedPaintDC {
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
