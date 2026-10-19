//! WinUI3 Menu System
//!
//! This module provides menu types: MenuBar, Menu, and PopupMenu.

use std::cell::RefCell;
use std::collections::HashMap;

use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

/// Menu item type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuItemKind {
    Normal,
    Separator,
    Check,
    Radio,
}

thread_local! {
    static MENU_CALLBACKS: RefCell<HashMap<u16, Box<dyn Fn()>>> = RefCell::new(HashMap::new());
}

/// Menu item handle wrapper
pub struct MenuItemHandle(pub HMENU);

/// Menu item identifier
pub struct MenuId(pub u16);

impl MenuId {
    pub fn new() -> Self {
        static mut ID_COUNTER: u16 = 1000;
        unsafe {
            ID_COUNTER += 1;
            Self(ID_COUNTER)
        }
    }
}

impl Default for MenuId {
    fn default() -> Self {
        Self::new()
    }
}

/// Individual menu item
pub struct MenuItem {
    id: u16,
    label: String,
    kind: MenuItemKind,
    enabled: bool,
    checked: bool,
    submenu: Option<Menu>,
}

impl MenuItem {
    pub fn new(id: u16, label: &str, kind: MenuItemKind) -> Self {
        Self {
            id,
            label: label.to_string(),
            kind,
            enabled: true,
            checked: false,
            submenu: None,
        }
    }

    pub fn separator() -> Self {
        Self::new(0, "", MenuItemKind::Separator)
    }

    pub fn text(id: u16, label: &str) -> Self {
        Self::new(id, label, MenuItemKind::Normal)
    }

    pub fn check(id: u16, label: &str) -> Self {
        Self::new(id, label, MenuItemKind::Check)
    }

    pub fn radio(id: u16, label: &str) -> Self {
        Self::new(id, label, MenuItemKind::Radio)
    }
}

/// Menu (popup or submenu)
pub struct Menu {
    hmenu: HMENU,
    items: Vec<MenuItem>,
    callbacks: HashMap<u16, Box<dyn Fn()>>,
}

impl Menu {
    pub fn new() -> Self {
        unsafe {
            let hmenu = CreateMenu();
            Self {
                hmenu,
                items: Vec::new(),
                callbacks: HashMap::new(),
            }
        }
    }

    pub fn new_popup() -> Self {
        unsafe {
            let hmenu = CreatePopupMenu();
            Self {
                hmenu,
                items: Vec::new(),
                callbacks: HashMap::new(),
            }
        }
    }

    pub fn hmenu(&self) -> HMENU {
        self.hmenu
    }

    pub fn append(&mut self, item: MenuItem) {
        unsafe {
            match item.kind {
                MenuItemKind::Separator => {
                    AppendMenuW(self.hmenu, MF_SEPARATOR, 0, std::ptr::null());
                }
                MenuItemKind::Normal => {
                    let label = if item.label.is_empty() {
                        None
                    } else {
                        Some(to_wide(&item.label))
                    };
                    AppendMenuW(
                        self.hmenu,
                        MF_STRING,
                        item.id as usize,
                        label.map_or(std::ptr::null(), |w| w.as_ptr()),
                    );
                }
                MenuItemKind::Check => {
                    let mut flags = MF_STRING | MF_CHECKED;
                    if !item.enabled {
                        flags |= MF_GRAYED;
                    }
                    let label = to_wide(&item.label);
                    AppendMenuW(self.hmenu, flags, item.id as usize, label.as_ptr());
                }
                MenuItemKind::Radio => {
                    let flags = MF_STRING;
                    let label = to_wide(&item.label);
                    AppendMenuW(self.hmenu, flags, item.id as usize, label.as_ptr());
                }
            }
        }
        self.items.push(item);
    }

    pub fn append_separator(&mut self) {
        self.append(MenuItem::separator());
    }

    pub fn append_text(&mut self, id: u16, label: &str) {
        self.append(MenuItem::text(id, label));
    }

    pub fn append_check(&mut self, id: u16, label: &str) {
        self.append(MenuItem::check(id, label));
    }

    pub fn append_radio(&mut self, id: u16, label: &str) {
        self.append(MenuItem::radio(id, label));
    }

    // Stub implementations for API compatibility
    pub fn append_with_svg_icon(&mut self, _id: u16, _label: &str, _svg_data: &[u8], _size: u32) {
        // SVG icons not yet supported in WinUI3 - fall back to text
        self.append_text(_id, _label);
    }

    pub fn append_disabled(&mut self, _id: u16, _label: &str) {
        // Disabled items not yet supported in WinUI3
        self.append_text(_id, _label);
    }

    pub fn append_submenu(&mut self, _id: u16, label: &str, submenu: &mut Menu) {
        unsafe {
            let wide_label = to_wide(label);
            AppendMenuW(
                self.hmenu,
                MF_POPUP,
                submenu.hmenu as usize,
                wide_label.as_ptr(),
            );
        }
    }

    pub fn enable(&mut self, id: u16, enabled: bool) {
        unsafe {
            let flags = if enabled { MF_ENABLED } else { MF_GRAYED };
            EnableMenuItem(self.hmenu, id as u32, MF_BYCOMMAND | flags);
        }
    }

    pub fn check_item(&mut self, id: u16, checked: bool) {
        unsafe {
            let flags = if checked { MF_CHECKED } else { MF_UNCHECKED };
            CheckMenuItem(self.hmenu, id as u32, MF_BYCOMMAND | flags);
        }
    }

    pub fn get_item_count(&self) -> usize {
        self.items.len()
    }
}

impl Drop for Menu {
    fn drop(&mut self) {
        unsafe {
            DestroyMenu(self.hmenu);
        }
    }
}

impl Default for Menu {
    fn default() -> Self {
        Self::new()
    }
}

/// Menu bar (main window menu)
pub struct MenuBar {
    hmenu: HMENU,
    menus: Vec<(String, Menu)>,
}

impl MenuBar {
    pub fn new() -> Self {
        unsafe {
            let hmenu = CreateMenu();
            Self {
                hmenu,
                menus: Vec::new(),
            }
        }
    }

    pub fn hmenu(&self) -> HMENU {
        self.hmenu
    }

    pub fn append(&mut self, label: &str, menu: Menu) {
        unsafe {
            let wide_label = to_wide(label);
            AppendMenuW(
                self.hmenu,
                MF_POPUP,
                menu.hmenu() as usize,
                wide_label.as_ptr(),
            );
        }
        self.menus.push((label.to_string(), menu));
    }

    pub fn get_menu_count(&self) -> usize {
        self.menus.len()
    }

    pub fn enable(&mut self, _index: usize, _enabled: bool) {
        // Enable/disable entire menu
        // This would require EnableMenuItem with proper flags
    }
}

impl Drop for MenuBar {
    fn drop(&mut self) {
        unsafe {
            DestroyMenu(self.hmenu);
        }
    }
}

impl Default for MenuBar {
    fn default() -> Self {
        Self::new()
    }
}

/// PopupMenu (context menu)
pub struct PopupMenu {
    menu: Menu,
}

impl PopupMenu {
    pub fn new() -> Self {
        Self {
            menu: Menu::new_popup(),
        }
    }

    pub fn append(&mut self, item: MenuItem) {
        self.menu.append(item);
    }

    pub fn append_separator(&mut self) {
        self.menu.append_separator();
    }

    pub fn append_text(&mut self, id: u16, label: &str) {
        self.menu.append_text(id, label);
    }

    pub fn append_check(&mut self, id: u16, label: &str) {
        self.menu.append_check(id, label);
    }

    pub fn append_radio(&mut self, id: u16, label: &str) {
        self.menu.append_radio(id, label);
    }

    #[allow(clippy::not_unsafe_ptr_arg_deref)] // thin FFI wrapper; `hwnd` must be live
    pub fn popup(&self, hwnd: HWND, x: i32, y: i32) -> Option<u16> {
        unsafe {
            let result = TrackPopupMenu(
                self.menu.hmenu(),
                TPM_LEFTALIGN | TPM_TOPALIGN | TPM_RETURNCMD,
                x,
                y,
                0,
                hwnd,
                std::ptr::null(),
            );
            if result == 0 {
                None
            } else {
                Some(result as u16)
            }
        }
    }
}

impl Default for PopupMenu {
    fn default() -> Self {
        Self::new()
    }
}

// Helper function
fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}
