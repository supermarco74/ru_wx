//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Menu open/close events (`wxMenuEvent`).

/// Delivered when a menu is about to open or has closed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MenuEvent {
    pub menu_id: u16,
    pub is_popup: bool,
    pub opening: bool,
}

impl MenuEvent {
    pub const fn opening(menu_id: u16, is_popup: bool) -> Self {
        Self {
            menu_id,
            is_popup,
            opening: true,
        }
    }

    pub const fn closed(menu_id: u16, is_popup: bool) -> Self {
        Self {
            menu_id,
            is_popup,
            opening: false,
        }
    }
}
