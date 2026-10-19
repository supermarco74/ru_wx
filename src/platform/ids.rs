//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Cross-platform control / menu id allocation.

use std::sync::atomic::{AtomicU16, AtomicU32, Ordering};

static NEXT_CONTROL_ID: AtomicU16 = AtomicU16::new(100);
static NEXT_MENU_ID: AtomicU16 = AtomicU16::new(9000);
static NEXT_TIMER_ID: AtomicU32 = AtomicU32::new(1);

/// Unique child-control id (shared by Win32, AppKit, and GTK stub backends).
pub fn next_control_id() -> u16 {
    let id = NEXT_CONTROL_ID.fetch_add(1, Ordering::Relaxed);
    if id >= 9000 {
        panic!("ru_wx: control ID space exhausted (IDs must stay below 9000)");
    }
    id
}

/// Unique AppKit timer id (`NSTimer` callbacks).
pub fn next_timer_id() -> u32 {
    NEXT_TIMER_ID.fetch_add(1, Ordering::Relaxed)
}

/// Unique menu-item id.
pub fn next_menu_id() -> u16 {
    NEXT_MENU_ID.fetch_add(1, Ordering::Relaxed)
}
