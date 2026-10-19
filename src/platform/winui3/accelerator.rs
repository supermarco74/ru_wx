//! `Accelerator` and `AcceleratorTable` — keyboard shortcut helpers.
//!
//! The WinUI3 backend models an accelerator as a tuple of
//! `(MOD_ALT | MOD_CONTROL | MOD_SHIFT | MOD_WIN, virtual_key, id)`
//! stored in a per-frame `HACCEL`. On every `GetMessageW`, the
//! accelerator table is consulted before dispatching the message;
//! if it matches, the message is converted into a `WM_COMMAND`
//! against the matching id.
//!
//! The public surface mirrors the Win32 API:
//!
//! * [`AcceleratorKey::parse`] — parse `"Ctrl+S"` / `"Alt+Shift+F4"`
//!   / `"F11"` into a structured spec.
//! * [`AcceleratorTable::new`] — register a list of [`AcceleratorKey`]s.
//! * [`AcceleratorTable::translate`](crate::platform::winui3::accelerator::AcceleratorTable::translate) — call this from the
//!   message-pump before `TranslateMessage`.

#![allow(non_snake_case)]

use std::cell::RefCell;
use std::collections::HashMap;

use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

pub const MOD_ALT: u32 = 0x0001;
pub const MOD_CONTROL: u32 = 0x0002;
pub const MOD_SHIFT: u32 = 0x0004;
pub const MOD_WIN: u32 = 0x0008;

pub struct AcceleratorKey {
    pub mods: u32,
    pub vk: u32,
    pub id: u16,
}

impl AcceleratorKey {
    /// Parse a shortcut spec such as `"Ctrl+S"` or `"Alt+Shift+F4"`
    /// or `"F11"`.
    ///
    /// Returns `None` on parse failure. The id is supplied separately
    /// because shortcut-vs-id mapping is application-defined.
    pub fn parse(spec: &str, id: u16) -> Option<Self> {
        let mut mods: u32 = 0;
        let mut key_part = spec.trim();
        for token in key_part.split('+') {
            let token = token.trim();
            match token.to_ascii_lowercase().as_str() {
                "ctrl" | "control" => mods |= MOD_CONTROL,
                "alt" => mods |= MOD_ALT,
                "shift" => mods |= MOD_SHIFT,
                "win" | "meta" | "cmd" => mods |= MOD_WIN,
                _ => {
                    key_part = token;
                    break;
                }
            }
        }
        let vk = virtual_key_from_name(key_part)?;
        Some(Self { mods, vk, id })
    }
}

fn virtual_key_from_name(name: &str) -> Option<u32> {
    if name.len() == 1 {
        let upper = name.to_ascii_uppercase();
        if let Some(ch) = upper.chars().next() {
            if ch.is_ascii_alphabetic() || ch.is_ascii_digit() {
                return Some(ch as u32);
            }
        }
    }
    match name.to_ascii_uppercase().as_str() {
        "F1" => Some(0x70),
        "F2" => Some(0x71),
        "F3" => Some(0x72),
        "F4" => Some(0x73),
        "F5" => Some(0x74),
        "F6" => Some(0x75),
        "F7" => Some(0x76),
        "F8" => Some(0x77),
        "F9" => Some(0x78),
        "F10" => Some(0x79),
        "F11" => Some(0x7A),
        "F12" => Some(0x7B),
        "ESC" | "ESCAPE" => Some(0x1B),
        "ENTER" | "RETURN" => Some(0x0D),
        "TAB" => Some(0x09),
        "SPACE" => Some(0x20),
        "DELETE" | "DEL" => Some(0x2E),
        "INSERT" | "INS" => Some(0x2D),
        "HOME" => Some(0x24),
        "END" => Some(0x23),
        "PAGEUP" | "PGUP" => Some(0x21),
        "PAGEDOWN" | "PGDN" => Some(0x22),
        "LEFT" => Some(0x25),
        "UP" => Some(0x26),
        "RIGHT" => Some(0x27),
        "DOWN" => Some(0x28),
        _ => None,
    }
}

// ============================================================================
// Accelerator table.
//
// Each Frame owns at most one `AcceleratorTable`. The handle is a
// `HACCEL` allocated by the Win32 `CreateAcceleratorTableW` call;
// it is freed in `Drop`.
pub struct AcceleratorTable {
    handle: HACCEL,
    entries: Vec<AcceleratorEntry>,
}

#[derive(Copy, Clone)]
pub struct AcceleratorEntry {
    pub mods: u32,
    pub vk: u32,
    pub id: u16,
}

impl AcceleratorTable {
    pub fn new(keys: &[AcceleratorKey]) -> Self {
        if keys.is_empty() {
            return Self {
                handle: std::ptr::null_mut(),
                entries: Vec::new(),
            };
        }
        // Translate our structs into the Win32 ACCEL struct. The Win32
        // `fVirt` field uses *different* bits than the accelerator
        // modifier API: FALT = 0x10, FCONTROL = 0x08, FSHIFT = 0x04,
        // FVIRTKEY = 0x01. Our `AcceleratorKey::mods` uses the
        // accelerator API convention, so we translate.
        let mut raw: Vec<ACCEL> = keys
            .iter()
            .map(|k| {
                let mut fvirt: u8 = 0x01; // FVIRTKEY — we always use VK codes.
                if (k.mods & MOD_ALT) != 0 {
                    fvirt |= 0x10;
                }
                if (k.mods & MOD_CONTROL) != 0 {
                    fvirt |= 0x08;
                }
                if (k.mods & MOD_SHIFT) != 0 {
                    fvirt |= 0x04;
                }
                ACCEL {
                    fVirt: fvirt,
                    key: k.vk as u16,
                    cmd: k.id,
                }
            })
            .collect();
        let handle = unsafe { CreateAcceleratorTableW(raw.as_mut_ptr(), raw.len() as i32) };
        let entries = raw
            .iter()
            .map(|a| AcceleratorEntry {
                mods: a.fVirt as u32,
                vk: a.key as u32,
                id: a.cmd,
            })
            .collect();
        Self { handle, entries }
    }

    /// Call before `TranslateMessage`. Returns `true` if the message was
    /// translated (the caller should skip `TranslateMessage` /
    /// `DispatchMessageW` and let `WM_COMMAND` propagate naturally).
    pub fn translate(&self, msg: &mut MSG) -> bool {
        if self.handle.is_null() {
            return false;
        }
        let translated = unsafe { TranslateAcceleratorW(msg.hwnd, self.handle, msg) };
        translated != 0
    }

    pub fn entries(&self) -> &[AcceleratorEntry] {
        &self.entries
    }

    pub fn handle(&self) -> HACCEL {
        self.handle
    }
}

impl Drop for AcceleratorTable {
    fn drop(&mut self) {
        if !self.handle.is_null() {
            // windows-sys 0.59 does not re-export DestroyAcceleratorTable.
            // SAFETY: `h` was returned by CreateAcceleratorTableW.
            unsafe {
                DestroyAcceleratorTable(self.handle);
            }
        }
    }
}

// External binding for `DestroyAcceleratorTable`.
// Windows-sys 0.59 doesn't export it; we declare it via the raw-dylib
// ABI so the loader resolves it at runtime. Note: Microsoft exports
// a single `DestroyAcceleratorTable` symbol (no `W` suffix) and the
// same function serves both ANSI and Unicode callers.
#[link(name = "user32.dll", kind = "raw-dylib", modifiers = "+verbatim")]
unsafe extern "system" {
    fn DestroyAcceleratorTable(h: HACCEL) -> i32;
}

// ============================================================================
// Per-thread map of `HWND -> AcceleratorTable` so the message pump
// can find the right table without each Frame having to hand it in.
thread_local! {
    static ACCEL_MAP: RefCell<HashMap<usize, *const AcceleratorTable>> = RefCell::new(HashMap::new());
}

pub fn attach(hwnd: HWND, table: &AcceleratorTable) {
    ACCEL_MAP.with(|m| {
        m.borrow_mut().insert(hwnd as usize, table as *const _);
    });
}

pub fn detach(hwnd: HWND) {
    ACCEL_MAP.with(|m| {
        m.borrow_mut().remove(&(hwnd as usize));
    });
}

pub fn with_table<F: FnOnce(&AcceleratorTable)>(hwnd: HWND, f: F) {
    ACCEL_MAP.with(|m| {
        let m = m.borrow();
        if let Some(ptr) = m.get(&(hwnd as usize)) {
            // SAFETY: we trust the caller to keep the table alive while it is attached.
            let table = unsafe { &**ptr };
            f(table);
        }
    });
}
