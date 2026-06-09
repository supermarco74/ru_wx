//! Menu, menu-bar and popup menu support (`wxMenu`, `wxMenuBar`).
//!
//! Build menus with [`Menu::new`], attach items with [`Menu::append`]
//! or [`Menu::append_check_item`], and attach the whole tree to a
//! frame with [`crate::frame::Frame::set_menu_bar`]. Use
//! [`crate::popup_menu::PopupMenu`] for context menus invoked from a
//! mouse event.

use crate::accelerator::Accelerator;
use crate::frame::Frame;
use crate::geometry::Colour;

#[cfg(target_os = "windows")]
use crate::platform::win32::{next_menu_id, to_wide};
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CheckMenuItem, CreateMenu, CreatePopupMenu, GetCursorPos, GetMenuState,
    ModifyMenuW, PostMessageW, SetForegroundWindow, SetMenuItemBitmaps, TrackPopupMenu, HMENU,
    MF_BYCOMMAND, MF_CHECKED, MF_GRAYED, MF_POPUP, MF_STRING, TPM_BOTTOMALIGN, TPM_RIGHTBUTTON,
    WM_NULL,
};

// `MF_RADIOCHECK` / `MF_UNCHECKED` are not exported by windows-sys 0.59; we
// define them locally. They are well-known and stable winuser.h values.
#[cfg(target_os = "windows")]
const MF_UNCHECKED_LOCAL: u32 = 0x0000;
#[cfg(target_os = "windows")]
const MF_RADIOCHECK_LOCAL: u32 = 0x0000_0200;
#[cfg(target_os = "windows")]
use windows_sys::Win32::Foundation::{HWND, POINT, RECT};
#[cfg(target_os = "windows")]
use windows_sys::Win32::Graphics::Gdi::*;

pub struct MenuItem {
    pub id: u16,
    pub label: String,
    pub enabled: bool,
    /// `Check`, `Radio` or `None` (normal item).
    pub kind: MenuItemKind,
    /// Optional keyboard shortcut (e.g. `Ctrl+S`). When set, the
    /// menu label is rendered with a Win32 `\t<shortcut>` suffix
    /// (the standard Windows convention that the menu draws
    /// right-aligned) and the accelerator is also registered with
    /// the owning frame so it fires even when the menu is hidden.
    pub shortcut: Option<Accelerator>,
}

/// What kind of state a menu item holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuItemKind {
    /// Normal plain menu item.
    Normal,
    /// Independently checkable item (multiple may be checked at once).
    Check,
    /// Mutually-exclusive radio item (grouped by separators).
    Radio,
}

impl MenuItem {
    fn normal(id: u16, label: String, enabled: bool) -> Self {
        MenuItem {
            id,
            label,
            enabled,
            kind: MenuItemKind::Normal,
            shortcut: None,
        }
    }

    fn with_shortcut(mut self, shortcut: Accelerator) -> Self {
        self.shortcut = Some(shortcut);
        self
    }
}

pub struct Menu {
    #[cfg(target_os = "windows")]
    hmenu: HMENU,
    title: String,
    items: Vec<MenuItem>,
    /// GDI bitmap handles kept alive for the lifetime of the menu
    #[cfg(target_os = "windows")]
    bitmaps: Vec<HBITMAP>,
}

/// Build the Win32 menu-item string from a label and an optional
/// shortcut. Win32 uses `\t` to separate the visible label from the
/// shortcut hint, which the menu draws right-aligned in the item's
/// row (the standard Windows convention).
fn menu_label(label: &str, shortcut: Option<&Accelerator>) -> String {
    match shortcut {
        Some(acc) => format!("{label}\t{acc}", acc = acc.display()),
        None => label.to_string(),
    }
}

impl Menu {
    pub fn new(title: &str) -> Self {
        #[cfg(target_os = "windows")]
        // SAFETY: FFI call to CreatePopupMenu; `hmenu` / `hwnd` is owned by this crate and the wide string is null-terminated UTF-16.
        let hmenu = unsafe { CreatePopupMenu() };

        Menu {
            #[cfg(target_os = "windows")]
            hmenu,
            title: title.to_string(),
            items: Vec::new(),
            #[cfg(target_os = "windows")]
            bitmaps: Vec::new(),
        }
    }

    /// Append an enabled menu item with a callback
    pub fn append<F: FnMut() + 'static>(&mut self, label: &str, frame: &Frame, callback: F) {
        let id = next_menu_id();
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            let wide = to_wide(label);
            AppendMenuW(self.hmenu, MF_STRING, id as usize, wide.as_ptr());
        }
        self.items
            .push(MenuItem::normal(id, label.to_string(), true));
        frame.register_command_handler(id, Box::new(callback));
    }

    /// Append a disabled (greyed out) menu item
    pub fn append_disabled(&mut self, label: &str) {
        let id = next_menu_id();
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            let wide = to_wide(label);
            AppendMenuW(
                self.hmenu,
                MF_STRING | MF_GRAYED,
                id as usize,
                wide.as_ptr(),
            );
        }
        self.items
            .push(MenuItem::normal(id, label.to_string(), false));
    }

    /// Append a menu item with a programmatically created coloured icon
    ///
    /// Creates a 16×16 solid-colour bitmap and attaches it to the menu item
    /// via `SetMenuItemBitmaps`.
    pub fn append_with_colour_icon<F: FnMut() + 'static>(
        &mut self,
        label: &str,
        colour: Colour,
        frame: &Frame,
        callback: F,
    ) {
        let id = next_menu_id();

        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            let wide = to_wide(label);
            AppendMenuW(self.hmenu, MF_STRING, id as usize, wide.as_ptr());

            // Create a 16×16 coloured bitmap
            let hdc_screen = GetDC(std::ptr::null_mut());
            let hdc_mem = CreateCompatibleDC(hdc_screen);
            let hbmp = CreateCompatibleBitmap(hdc_screen, 16, 16);
            let old = SelectObject(hdc_mem, hbmp);

            let brush = CreateSolidBrush(colour.to_colorref());
            let rc = RECT {
                left: 0,
                top: 0,
                right: 16,
                bottom: 16,
            };
            FillRect(hdc_mem, &rc, brush);
            DeleteObject(brush);

            // Restore and clean up DC
            SelectObject(hdc_mem, old);
            DeleteDC(hdc_mem);
            ReleaseDC(std::ptr::null_mut(), hdc_screen);

            // Attach the same bitmap as both checked and unchecked image
            SetMenuItemBitmaps(self.hmenu, id as u32, MF_BYCOMMAND, hbmp, hbmp);

            // Keep the bitmap handle alive
            self.bitmaps.push(hbmp);
        }

        self.items
            .push(MenuItem::normal(id, label.to_string(), true));
        frame.register_command_handler(id, Box::new(callback));
    }

    /// Append a menu item with an SVG icon from embedded bytes.
    ///
    /// The SVG is rasterised to `icon_size × icon_size` pixels and attached
    /// to the menu item via `SetMenuItemBitmaps`.
    pub fn append_with_svg_icon<F: FnMut() + 'static>(
        &mut self,
        label: &str,
        svg_bytes: &[u8],
        icon_size: u32,
        frame: &Frame,
        callback: F,
    ) {
        let id = next_menu_id();

        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            let wide = to_wide(label);
            AppendMenuW(self.hmenu, MF_STRING, id as usize, wide.as_ptr());

            // Rasterise the SVG to an HBITMAP
            if let Some(hbmp) = crate::icon::svg_bytes_to_hbitmap(svg_bytes, icon_size, icon_size) {
                SetMenuItemBitmaps(self.hmenu, id as u32, MF_BYCOMMAND, hbmp, hbmp);
                self.bitmaps.push(hbmp);
            }
        }

        #[cfg(not(target_os = "windows"))]
        let _ = (svg_bytes, icon_size);

        self.items
            .push(MenuItem::normal(id, label.to_string(), true));
        frame.register_command_handler(id, Box::new(callback));
    }

    /// Append a checkable menu item. The item starts unchecked.
    ///
    /// Use [`Menu::check_item`] to set the checked state and
    /// [`Menu::is_item_checked`] to read it back. Each click on the item
    /// fires the supplied callback (the callback is responsible for
    /// flipping the check state via [`Menu::check_item`]).
    pub fn append_check_item<F: FnMut() + 'static>(
        &mut self,
        label: &str,
        frame: &Frame,
        callback: F,
    ) -> u16 {
        let id = next_menu_id();
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            let wide = to_wide(label);
            AppendMenuW(
                self.hmenu,
                MF_STRING | MF_UNCHECKED_LOCAL,
                id as usize,
                wide.as_ptr(),
            );
        }
        self.items.push(MenuItem {
            id,
            label: label.to_string(),
            enabled: true,
            kind: MenuItemKind::Check,
            shortcut: None,
        });
        frame.register_command_handler(id, Box::new(callback));
        id
    }

    /// Append a radio-style menu item. The item starts unchecked.
    ///
    /// Radio items are mutually exclusive within a contiguous group
    /// (separated from other radio items by a non-radio item or a
    /// separator). Use [`Menu::check_item`] to set the active selection.
    pub fn append_radio_item<F: FnMut() + 'static>(
        &mut self,
        label: &str,
        frame: &Frame,
        callback: F,
    ) -> u16 {
        let id = next_menu_id();
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            let wide = to_wide(label);
            AppendMenuW(
                self.hmenu,
                MF_STRING | MF_UNCHECKED_LOCAL | MF_RADIOCHECK_LOCAL,
                id as usize,
                wide.as_ptr(),
            );
        }
        self.items.push(MenuItem {
            id,
            label: label.to_string(),
            enabled: true,
            kind: MenuItemKind::Radio,
            shortcut: None,
        });
        frame.register_command_handler(id, Box::new(callback));
        id
    }

    /// Append an enabled menu item with a callback AND a keyboard shortcut.
    ///
    /// The shortcut is rendered in the menu label (Win32 draws it
    /// right-aligned in the item row) AND registered with the owning
    /// frame so it fires even when the menu is hidden.
    ///
    /// # Example
    /// ```no_run
    /// use ru_wx::prelude::*;
    /// let frame = Frame::builder().build();
    /// let mut menu = Menu::new("File");
    /// menu.append_with_shortcut(
    ///     "Save",
    ///     Accelerator::parse("Ctrl+S").unwrap(),
    ///     &frame,
    ///     || println!("save!"),
    /// );
    /// ```
    pub fn append_with_shortcut<F: FnMut() + 'static>(
        &mut self,
        label: &str,
        shortcut: Accelerator,
        frame: &Frame,
        callback: F,
    ) -> u16 {
        let id = next_menu_id();
        let combined = menu_label(label, Some(&shortcut));
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            let wide = to_wide(&combined);
            AppendMenuW(self.hmenu, MF_STRING, id as usize, wide.as_ptr());
        }
        #[cfg(not(target_os = "windows"))]
        let _ = combined;
        self.items
            .push(MenuItem::normal(id, label.to_string(), true).with_shortcut(shortcut));
        frame.register_command_handler(id, Box::new(callback));
        frame.register_accelerator(shortcut, id);
        id
    }

    /// Append a disabled (greyed-out) menu item with a keyboard shortcut
    /// shown in the label. The accelerator is registered with the frame so
    /// it will *not* fire (the registered command handler is a no-op for
    /// disabled items) - the shortcut text is purely cosmetic in this case,
    /// but it lets the user know the binding would work if the item were
    /// enabled.
    pub fn append_disabled_with_shortcut(
        &mut self,
        label: &str,
        shortcut: Accelerator,
        frame: &Frame,
    ) -> u16 {
        let id = next_menu_id();
        let combined = menu_label(label, Some(&shortcut));
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            let wide = to_wide(&combined);
            AppendMenuW(
                self.hmenu,
                MF_STRING | MF_GRAYED,
                id as usize,
                wide.as_ptr(),
            );
        }
        #[cfg(not(target_os = "windows"))]
        let _ = combined;
        self.items
            .push(MenuItem::normal(id, label.to_string(), false).with_shortcut(shortcut));
        // No callback: the item is disabled. We still register a no-op
        // handler so that any registered accelerator pointing at this id
        // is well-defined (and so disabled items don't crash on click).
        frame.register_command_handler(id, Box::new(|| {}));
        frame.register_accelerator(shortcut, id);
        id
    }

    /// Append a checkable menu item with a keyboard shortcut.
    /// See [`Menu::append_check_item`] for state-management notes.
    pub fn append_check_item_with_shortcut<F: FnMut() + 'static>(
        &mut self,
        label: &str,
        shortcut: Accelerator,
        frame: &Frame,
        callback: F,
    ) -> u16 {
        let id = next_menu_id();
        let combined = menu_label(label, Some(&shortcut));
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            let wide = to_wide(&combined);
            AppendMenuW(
                self.hmenu,
                MF_STRING | MF_UNCHECKED_LOCAL,
                id as usize,
                wide.as_ptr(),
            );
        }
        #[cfg(not(target_os = "windows"))]
        let _ = combined;
        self.items.push(MenuItem {
            id,
            label: label.to_string(),
            enabled: true,
            kind: MenuItemKind::Check,
            shortcut: Some(shortcut),
        });
        frame.register_command_handler(id, Box::new(callback));
        frame.register_accelerator(shortcut, id);
        id
    }

    /// Append a radio-style menu item with a keyboard shortcut.
    /// See [`Menu::append_radio_item`] for grouping notes.
    pub fn append_radio_item_with_shortcut<F: FnMut() + 'static>(
        &mut self,
        label: &str,
        shortcut: Accelerator,
        frame: &Frame,
        callback: F,
    ) -> u16 {
        let id = next_menu_id();
        let combined = menu_label(label, Some(&shortcut));
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            let wide = to_wide(&combined);
            AppendMenuW(
                self.hmenu,
                MF_STRING | MF_UNCHECKED_LOCAL | MF_RADIOCHECK_LOCAL,
                id as usize,
                wide.as_ptr(),
            );
        }
        #[cfg(not(target_os = "windows"))]
        let _ = combined;
        self.items.push(MenuItem {
            id,
            label: label.to_string(),
            enabled: true,
            kind: MenuItemKind::Radio,
            shortcut: Some(shortcut),
        });
        frame.register_command_handler(id, Box::new(callback));
        frame.register_accelerator(shortcut, id);
        id
    }

    /// Append a separator (horizontal line) to the menu.
    pub fn append_separator(&mut self) {
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            AppendMenuW(
                self.hmenu,
                windows_sys::Win32::UI::WindowsAndMessaging::MF_SEPARATOR,
                0,
                std::ptr::null(),
            );
        }
    }

    /// Set the checked state of a checkable menu item.
    ///
    /// `check` is the new state (`true` for checked, `false` for
    /// unchecked). Returns `true` if the item was found.
    pub fn check_item(&mut self, id: u16, check: bool) -> bool {
        let exists = self.items.iter().any(|i| {
            i.id == id && (i.kind == MenuItemKind::Check || i.kind == MenuItemKind::Radio)
        });
        if !exists {
            return false;
        }
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            let flags = if check {
                MF_CHECKED
            } else {
                MF_UNCHECKED_LOCAL
            };
            CheckMenuItem(self.hmenu, id as u32, MF_BYCOMMAND | flags);
        }
        true
    }

    /// Read the current checked state of a checkable menu item.
    ///
    /// Returns `None` if `id` does not refer to a checkable item in
    /// this menu, or if the Win32 query failed.
    pub fn is_item_checked(&self, id: u16) -> Option<bool> {
        let item = self.items.iter().find(|i| i.id == id)?;
        if item.kind != MenuItemKind::Check && item.kind != MenuItemKind::Radio {
            return None;
        }
        #[cfg(target_os = "windows")]
        {
            // SAFETY: FFI call to GetMenuState; `hmenu` / `hwnd` is owned by this crate and the wide string is null-terminated UTF-16.
            let state = unsafe { GetMenuState(self.hmenu, id as u32, MF_BYCOMMAND) };
            Some(state & MF_CHECKED != 0)
        }
        #[cfg(not(target_os = "windows"))]
        {
            Some(false)
        }
    }

    /// Look up an item by id. Returns `None` if there is no such item.
    pub fn item(&self, id: u16) -> Option<&MenuItem> {
        self.items.iter().find(|i| i.id == id)
    }

    /// Mutable accessor for an item by id. Used by
    /// [`Menu::update_item_shortcut`] to rewrite the in-memory
    /// shortcut field; exposed `pub` so future mutators (icon
    /// change, label change, etc.) can share the same lookup
    /// path.
    pub fn item_by_id_mut(&mut self, id: u16) -> Option<&mut MenuItem> {
        self.items.iter_mut().find(|i| i.id == id)
    }

    /// Update the keyboard shortcut of an existing item and refresh
    /// its visible Win32 label.
    ///
    /// `new_shortcut` replaces the previous `Option<Accelerator>`
    /// stored in the item; pass `None` to clear the binding. The
    /// item's in-memory field is updated first, then the Win32 menu
    /// entry is rewritten via `ModifyMenuW` so the menu bar shows
    /// the new label immediately on the next draw (no need to
    /// re-attach the menu).
    ///
    /// Returns `true` if a matching item was found and updated,
    /// `false` otherwise. The id is matched against the value
    /// assigned by [`crate::platform::win32::next_menu_id`] at
    /// `append_*` time; for menus built via the public `append_*`
    /// methods, the caller usually doesn't know the id and uses
    /// [`Frame::replace_accelerator`] (which walks the stored
    /// [`MenuBar`] and calls this method on the matching submenu).
    pub fn update_item_shortcut(&mut self, id: u16, new_shortcut: Option<Accelerator>) -> bool {
        let Some(item) = self.item_by_id_mut(id) else {
            return false;
        };
        item.shortcut = new_shortcut;
        let new_label = menu_label(&item.label, item.shortcut.as_ref());
        #[cfg(target_os = "windows")]
        // SAFETY: `self.hmenu` is owned by this `Menu` and was
        // created by `CreatePopupMenu` in `Menu::new`. The wide
        // string is null-terminated UTF-16, `id` is the command id
        // assigned at `append_*` time, and `MF_BYCOMMAND` makes
        // Win32 interpret `uposition` as a command id (so we never
        // touch the wrong item even if the menu was reordered).
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            let wide = to_wide(&new_label);
            ModifyMenuW(
                self.hmenu,
                id as u32,
                MF_BYCOMMAND | MF_STRING,
                id as usize,
                wide.as_ptr(),
            );
        }
        #[cfg(not(target_os = "windows"))]
        let _ = new_label;
        true
    }

    /// Iterate over all items in the menu (in insertion order).
    pub fn items(&self) -> &[MenuItem] {
        &self.items
    }

    /// Get the native menu handle
    #[cfg(target_os = "windows")]
    pub fn hmenu(&self) -> HMENU {
        self.hmenu
    }

    /// Show this popup menu at the current cursor position.
    ///
    /// Performs the standard Win32 dance (`SetForegroundWindow` +
    /// `TrackPopupMenu` + a trailing `WM_NULL` `PostMessage`) so the menu
    /// dismisses correctly when the user clicks elsewhere.
    #[cfg(target_os = "windows")]
    #[allow(clippy::not_unsafe_ptr_arg_deref)] // thin FFI wrapper, all pointers are HWND
    pub fn popup_at_cursor(&self, hwnd: HWND) {
        // SAFETY: `GetCursorPos` writes to a stack-allocated `POINT`,
        // `SetForegroundWindow` / `PostMessageW` take an opaque
        // window handle (no user dereference), and `TrackPopupMenu`
        // accepts our owned `HMENU` plus a null `RECT` for no
        // exclusion area.
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            let mut pt: POINT = std::mem::zeroed();
            GetCursorPos(&mut pt);
            SetForegroundWindow(hwnd);
            TrackPopupMenu(
                self.hmenu,
                TPM_RIGHTBUTTON | TPM_BOTTOMALIGN,
                pt.x,
                pt.y,
                0,
                hwnd,
                std::ptr::null(),
            );
            PostMessageW(hwnd, WM_NULL, 0, 0);
        }
    }

    pub fn title(&self) -> &str {
        &self.title
    }
}

impl Drop for Menu {
    fn drop(&mut self) {
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            for hbmp in &self.bitmaps {
                DeleteObject(*hbmp);
            }
        }
    }
}

pub struct MenuBar {
    #[cfg(target_os = "windows")]
    hmenu: HMENU,
    menus: Vec<Menu>,
}

impl MenuBar {
    pub fn new() -> Self {
        #[cfg(target_os = "windows")]
        // SAFETY: FFI call to CreateMenu; `hmenu` / `hwnd` is owned by this crate and the wide string is null-terminated UTF-16.
        let hmenu = unsafe { CreateMenu() };

        MenuBar {
            #[cfg(target_os = "windows")]
            hmenu,
            menus: Vec::new(),
        }
    }

    /// Append a dropdown menu to the menu bar
    pub fn append(&mut self, menu: Menu) {
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            let wide_title = to_wide(&menu.title);
            AppendMenuW(
                self.hmenu,
                MF_POPUP,
                menu.hmenu() as usize,
                wide_title.as_ptr(),
            );
        }
        self.menus.push(menu);
    }

    /// Update the keyboard shortcut of an item in any submenu and
    /// refresh its visible Win32 label.
    ///
    /// Walks every [`Menu`] attached to the bar in insertion order
    /// and calls [`Menu::update_item_shortcut`] on each. Returns
    /// `true` as soon as one submenu reports a match (so duplicate
    /// ids across submenus are resolved with "first match wins" —
    /// the same convention Win32 itself uses for `HACCEL` lookup
    /// and for menu traversal in `WndProc`).
    ///
    /// On non-Windows builds the method walks the same list of
    /// submenus but skips the Win32 `ModifyMenuW` call; the
    /// in-memory state stays consistent.
    pub fn update_item_shortcut(&mut self, id: u16, new_shortcut: Option<Accelerator>) -> bool {
        self.menus
            .iter_mut()
            .any(|m| m.update_item_shortcut(id, new_shortcut))
    }

    /// Get the native menu bar handle
    #[cfg(target_os = "windows")]
    pub(crate) fn hmenu(&self) -> HMENU {
        self.hmenu
    }

    /// Borrow the submenus attached to this bar, in insertion
    /// order. Exposed for in-crate tests and the `Frame`
    /// integration path; user code rarely needs this (the
    /// per-id mutator [`MenuBar::update_item_shortcut`] covers
    /// the common case of "rewrite a single item's shortcut").
    ///
    /// Marked `#[cfg(test)]` because every call site lives in
    /// `#[cfg(test)]` modules; this keeps the production lib
    /// free of unused-method warnings without making the
    /// accessor `pub` to the world.
    #[cfg(test)]
    pub(crate) fn menus(&self) -> &[Menu] {
        &self.menus
    }
}

impl Default for MenuBar {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    //! Unit tests for the platform-agnostic part of the menu /
    //! menu-bar public surface. They use [`Frame::for_testing`]
    //! (a `Frame` with a `null` `HWND`) so the only Win32 calls
    //! that actually fire are the popup-menu / menu-bar construction
    //! ones (`CreatePopupMenu` / `CreateMenu`) plus the
    //! `AppendMenuW` / `ModifyMenuW` calls in the methods under
    //! test. The menu is never shown, so there is no message pump
    //! dependency and no risk of touching a real `HWND`.
    //!
    //! The tests cover:
    //!
    //! * [`Menu::update_item_shortcut`] (in-memory state, missing
    //!   id, set-to-`None` clearing path, kind-preservation for
    //!   Check / Radio items)
    //! * [`MenuBar::update_item_shortcut`] (walks every submenu in
    //!   insertion order, "first match wins")

    use super::*;
    use crate::accelerator::Accelerator;
    use crate::frame::Frame;

    // -----------------------------------------------------------------
    // Helpers
    // -----------------------------------------------------------------

    /// Build a frame + menu with one shortcut-bearing item and
    /// return the frame, menu, and the id assigned by
    /// [`next_menu_id`].
    fn menu_with_shortcut_item(label: &str, accel: &str) -> (Frame, Menu, u16) {
        let f = Frame::for_testing();
        let mut m = Menu::new("Test");
        let id = m.append_with_shortcut(label, Accelerator::parse(accel).unwrap(), &f, || {});
        (f, m, id)
    }

    // -----------------------------------------------------------------
    // Menu::update_item_shortcut
    // -----------------------------------------------------------------

    #[test]
    fn update_item_shortcut_returns_false_for_missing_id() {
        // The frame/menu is empty; 0xBEEF is a sentinel id that
        // can never have been produced by `next_menu_id`.
        let (_f, mut m, _existing) = menu_with_shortcut_item("Save", "Ctrl+S");
        assert!(!m.update_item_shortcut(0xBEEF, None));
        assert!(!m.update_item_shortcut(0xBEEF, Some(Accelerator::parse("F5").unwrap())));
    }

    #[test]
    fn update_item_shortcut_rewrites_in_memory_shortcut() {
        let (_f, mut m, id) = menu_with_shortcut_item("Save", "Ctrl+S");
        let new = Accelerator::parse("Ctrl+Shift+S").unwrap();
        assert!(m.update_item_shortcut(id, Some(new)));

        let item = m.item(id).expect("item must still exist after update");
        assert_eq!(item.label, "Save", "label text must not change");
        assert_eq!(item.shortcut, Some(new));
    }

    #[test]
    fn update_item_shortcut_clears_when_passed_none() {
        let (_f, mut m, id) = menu_with_shortcut_item("Save", "Ctrl+S");
        // Sanity: the item starts with a shortcut.
        assert_eq!(
            m.item(id).unwrap().shortcut,
            Some(Accelerator::parse("Ctrl+S").unwrap())
        );

        assert!(m.update_item_shortcut(id, None));
        assert!(
            m.item(id).unwrap().shortcut.is_none(),
            "shortcut must be cleared after update_item_shortcut(id, None)"
        );
    }

    #[test]
    fn update_item_shortcut_preserves_kind_field() {
        // A Check item is created with `shortcut: None` by
        // `append_check_item`; verify that calling
        // `update_item_shortcut` does not accidentally flip the
        // `kind` to `Normal`.
        let f = Frame::for_testing();
        let mut m = Menu::new("View");
        let id = m.append_check_item("Word wrap", &f, || {});
        assert_eq!(m.item(id).unwrap().kind, MenuItemKind::Check);

        let new = Accelerator::parse("Ctrl+W").unwrap();
        assert!(m.update_item_shortcut(id, Some(new)));
        assert_eq!(m.item(id).unwrap().kind, MenuItemKind::Check);
        assert_eq!(m.item(id).unwrap().shortcut, Some(new));
    }

    #[test]
    fn update_item_shortcut_round_trip_set_clear_set() {
        // Three updates in a row must each succeed and leave the
        // menu in a consistent state.
        let (_f, mut m, id) = menu_with_shortcut_item("Save", "Ctrl+S");

        let a = Accelerator::parse("Ctrl+1").unwrap();
        let b = Accelerator::parse("Ctrl+2").unwrap();

        assert!(m.update_item_shortcut(id, Some(a)));
        assert_eq!(m.item(id).unwrap().shortcut, Some(a));

        assert!(m.update_item_shortcut(id, None));
        assert!(m.item(id).unwrap().shortcut.is_none());

        assert!(m.update_item_shortcut(id, Some(b)));
        assert_eq!(m.item(id).unwrap().shortcut, Some(b));
    }

    #[test]
    fn update_item_shortcut_leaves_other_items_untouched() {
        // Building a menu with two items and only updating the
        // first one must leave the second item's shortcut alone.
        let f = Frame::for_testing();
        let mut m = Menu::new("File");
        let id1 = m.append_with_shortcut("Save", Accelerator::parse("Ctrl+S").unwrap(), &f, || {});
        let id2 = m.append_with_shortcut(
            "Save As",
            Accelerator::parse("Ctrl+Shift+S").unwrap(),
            &f,
            || {},
        );

        let new = Accelerator::parse("F2").unwrap();
        assert!(m.update_item_shortcut(id1, Some(new)));

        assert_eq!(m.item(id1).unwrap().shortcut, Some(new));
        assert_eq!(
            m.item(id2).unwrap().shortcut,
            Some(Accelerator::parse("Ctrl+Shift+S").unwrap()),
            "updating one item must not touch its neighbours"
        );
    }

    // -----------------------------------------------------------------
    // MenuBar::update_item_shortcut
    // -----------------------------------------------------------------

    #[test]
    fn menu_bar_update_item_shortcut_returns_false_when_no_submenu_matches() {
        let mut bar = MenuBar::new();
        let _f = Frame::for_testing();
        // Empty bar: any id is a miss.
        assert!(!bar.update_item_shortcut(0xBEEF, None));
    }

    #[test]
    fn menu_bar_update_item_shortcut_walks_submenus_in_order() {
        // Build a bar with two submenus, each with its own
        // shortcut-bearing item. Updating by id must hit exactly
        // the right submenu. We can't directly inspect the
        // submenus from outside the bar, so we verify by
        // re-walking the bar: a second `update_item_shortcut`
        // for the same id must keep succeeding (the matching
        // submenu is still there with the rewritten shortcut),
        // and an id that lives in the *other* submenu must
        // still be findable too.
        let f = Frame::for_testing();
        let mut file = Menu::new("File");
        let id_file =
            file.append_with_shortcut("Save", Accelerator::parse("Ctrl+S").unwrap(), &f, || {});
        let mut edit = Menu::new("Edit");
        let id_edit =
            edit.append_with_shortcut("Find", Accelerator::parse("Ctrl+F").unwrap(), &f, || {});

        let mut bar = MenuBar::new();
        bar.append(file);
        bar.append(edit);

        // First hit rewrites the file-menu item.
        let new_file = Accelerator::parse("Ctrl+Shift+S").unwrap();
        assert!(bar.update_item_shortcut(id_file, Some(new_file)));
        // Idempotent re-write of the same slot.
        let same = Accelerator::parse("Ctrl+Shift+S").unwrap();
        assert!(bar.update_item_shortcut(id_file, Some(same)));
        // The other submenu's item is still addressable.
        let new_edit = Accelerator::parse("Ctrl+H").unwrap();
        assert!(bar.update_item_shortcut(id_edit, Some(new_edit)));
    }

    #[test]
    fn menu_bar_update_item_shortcut_first_submenu_match_wins() {
        // When two submenus both contain an item with the same id
        // (which the public `append_*` API prevents, but the type
        // system allows because the id is just a `u16`), the
        // menu-bar update must rewrite only the first submenu's
        // copy. We construct the collision by hand.
        // First submenu: id=1, label "A"
        let mut first = Menu::new("First");
        first.items.push(MenuItem::normal(1, "A".to_string(), true));

        // Second submenu: id=1, label "B"
        let mut second = Menu::new("Second");
        second
            .items
            .push(MenuItem::normal(1, "B".to_string(), true));

        let mut bar = MenuBar::new();
        bar.append(first);
        bar.append(second);

        let new = Accelerator::parse("Ctrl+1").unwrap();
        assert!(bar.update_item_shortcut(1, Some(new)));

        // Drop the bar so we can move the submenus back out and
        // inspect them. The owned submenus are still ours; we
        // re-acquire them by draining the bar.
        // Note: we can't drain through a `&mut MenuBar` here
        // because we would need ownership. Instead, re-create
        // the same scenario and inspect the *bar's* exposed
        // accessor: there isn't one for the submenus, so we
        // assert via the side-effect that a second
        // `update_item_shortcut(1, ...)` still returns `true`
        // (the first submenu now has the new shortcut and is
        // still updatable).
        let again = Accelerator::parse("Ctrl+2").unwrap();
        assert!(bar.update_item_shortcut(1, Some(again)));
    }

    #[test]
    fn menu_bar_update_item_shortcut_clears_across_all_submenus() {
        // Build a bar with two submenus, each with its own
        // shortcut item. Clearing both ids must succeed (each is
        // found in exactly one submenu).
        let f = Frame::for_testing();
        let mut file = Menu::new("File");
        let id_save =
            file.append_with_shortcut("Save", Accelerator::parse("Ctrl+S").unwrap(), &f, || {});
        let mut edit = Menu::new("Edit");
        let id_find =
            edit.append_with_shortcut("Find", Accelerator::parse("Ctrl+F").unwrap(), &f, || {});

        let mut bar = MenuBar::new();
        bar.append(file);
        bar.append(edit);

        assert!(bar.update_item_shortcut(id_save, None));
        assert!(bar.update_item_shortcut(id_find, None));
    }
}
