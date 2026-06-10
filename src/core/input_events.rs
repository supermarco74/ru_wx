//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Input and window events (`wxKeyEvent`, `wxMouseEvent`, …).
//!
//! Register on a [`crate::Frame`] via [`crate::Frame::on_key_down`],
//! [`crate::Frame::on_mouse`], etc.

use crate::core::geometry::{Point, Size};

/// Keyboard event (`wxKeyEvent`).
#[derive(Debug, Clone, Copy)]
pub struct KeyEvent {
    pub key_code: u32,
    pub modifiers: KeyModifiers,
    pub repeat_count: u16,
    pub is_sys_key: bool,
}

impl KeyEvent {
    pub(crate) fn from_win32(key_code: u32, lparam: isize, sys: bool) -> Self {
        let repeat_count = (lparam & 0xFFFF) as u16;
        let mut modifiers = KeyModifiers::empty();
        if (lparam >> 16) & 0x4000 != 0 {
            modifiers.insert(KeyModifiers::ALT);
        }
        if (lparam >> 16) & 0x2000 != 0 {
            modifiers.insert(KeyModifiers::PREVIOUS_STATE);
        }
        if (lparam >> 16) & 0x0100 != 0 {
            modifiers.insert(KeyModifiers::SHIFT);
        }
        if (lparam >> 16) & 0x8000 != 0 {
            modifiers.insert(KeyModifiers::EXTENDED);
        }
        Self {
            key_code,
            modifiers,
            repeat_count,
            is_sys_key: sys,
        }
    }

    pub fn control_down(&self) -> bool {
        self.modifiers.contains(KeyModifiers::CONTROL)
    }

    pub fn shift_down(&self) -> bool {
        self.modifiers.contains(KeyModifiers::SHIFT)
    }

    pub fn alt_down(&self) -> bool {
        self.modifiers.contains(KeyModifiers::ALT)
    }

    /// Attach control-key state from `GetKeyState(VK_CONTROL)`.
    pub(crate) fn with_control_state(mut self, control_down: bool) -> Self {
        if control_down {
            self.modifiers.insert(KeyModifiers::CONTROL);
        }
        self
    }
}

/// Modifier keys held during a key or mouse event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct KeyModifiers(u8);

impl KeyModifiers {
    pub const SHIFT: Self = Self(0x01);
    pub const CONTROL: Self = Self(0x02);
    pub const ALT: Self = Self(0x04);
    pub const EXTENDED: Self = Self(0x08);
    pub const PREVIOUS_STATE: Self = Self(0x10);

    pub const fn empty() -> Self {
        Self(0)
    }

    pub const fn contains(self, flag: Self) -> bool {
        self.0 & flag.0 != 0
    }

    pub const fn insert(&mut self, flag: Self) {
        self.0 |= flag.0;
    }
}

/// Mouse event kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseEventKind {
    LeftDown,
    LeftUp,
    RightDown,
    RightUp,
    MiddleDown,
    MiddleUp,
    Move,
    Wheel,
}

/// Mouse event (`wxMouseEvent`).
#[derive(Debug, Clone, Copy)]
pub struct MouseEvent {
    pub kind: MouseEventKind,
    pub position: Point,
    pub wheel_delta: i16,
    pub button_state: u16,
}

impl MouseEvent {
    pub(crate) fn from_win32(kind: MouseEventKind, lparam: isize, wparam: usize) -> Self {
        let x = (lparam & 0xFFFF) as i16 as i32;
        let y = ((lparam >> 16) & 0xFFFF) as i16 as i32;
        let wheel_delta = if kind == MouseEventKind::Wheel {
            ((wparam >> 16) & 0xFFFF) as i16
        } else {
            0
        };
        Self {
            kind,
            position: Point::new(x, y),
            wheel_delta,
            button_state: (wparam & 0xFFFF) as u16,
        }
    }

    pub fn left_is_down(&self) -> bool {
        self.button_state & 0x0001 != 0
    }
}

/// Focus gained or lost (`wxFocusEvent`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FocusEvent {
    pub gained: bool,
}

impl FocusEvent {
    pub const fn gained() -> Self {
        Self { gained: true }
    }

    pub const fn lost() -> Self {
        Self { gained: false }
    }
}

/// Size-change reason from `WM_SIZE` (`wxSizeEvent`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SizeType {
    Restored,
    Minimized,
    Maximized,
    ShowMaximized,
    ShowNormal,
    Other(u32),
}

impl SizeType {
    pub(crate) fn from_win32(wparam: usize) -> Self {
        match wparam {
            0 => Self::Restored,
            1 => Self::Minimized,
            2 => Self::Maximized,
            3 => Self::ShowMaximized,
            4 => Self::ShowNormal,
            other => Self::Other(other as u32),
        }
    }
}

/// Window resized (`wxSizeEvent`).
#[derive(Debug, Clone, Copy)]
pub struct SizeEvent {
    pub size: Size,
    pub size_type: SizeType,
}

impl SizeEvent {
    pub(crate) fn from_win32(wparam: usize, lparam: isize) -> Self {
        let width = (lparam & 0xFFFF) as i32;
        let height = ((lparam >> 16) & 0xFFFF) as i32;
        Self {
            size: Size::new(width, height),
            size_type: SizeType::from_win32(wparam),
        }
    }
}

/// Window moved (`wxMoveEvent`).
#[derive(Debug, Clone, Copy)]
pub struct MoveEvent {
    pub position: Point,
}

impl MoveEvent {
    pub(crate) fn from_win32(lparam: isize) -> Self {
        let x = (lparam & 0xFFFF) as i16 as i32;
        let y = ((lparam >> 16) & 0xFFFF) as i16 as i32;
        Self {
            position: Point::new(x, y),
        }
    }
}
