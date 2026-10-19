//! macOS menu-bar / popup check and radio item semantics.

use std::cell::Cell;
use std::collections::HashMap;

use crate::window::menu::MenuItemKind;

use super::{menu_set_item_checked, resolve_frame_handle, NativeHandle, FRAME_DATA_WEAK};

thread_local! {
    static MENU_ITEM_KINDS: std::cell::RefCell<HashMap<u16, MenuItemKind>> =
        std::cell::RefCell::new(HashMap::new());
    static MENU_ITEM_CHECKED: std::cell::RefCell<HashMap<u16, bool>> =
        std::cell::RefCell::new(HashMap::new());
    static MENU_ITEM_SUBMENU: std::cell::RefCell<HashMap<u16, u64>> =
        std::cell::RefCell::new(HashMap::new());
    static MENU_SUBMENU_LAYOUT: std::cell::RefCell<HashMap<u64, Vec<(u16, MenuItemKind)>>> =
        std::cell::RefCell::new(HashMap::new());
    static NEXT_SUBMENU_ID: Cell<u64> = Cell::new(1);
}

/// Row payload for [`super::install_menu_bar`] / [`super::show_popup_menu`].
pub type MenuInstallRow = (
    u16,
    String,
    bool,
    MenuItemKind,
    bool,
    Option<crate::core::accelerator::Accelerator>,
);

pub fn clear_menu_registry() {
    MENU_ITEM_KINDS.with(|m| m.borrow_mut().clear());
    MENU_ITEM_CHECKED.with(|m| m.borrow_mut().clear());
    MENU_ITEM_SUBMENU.with(|m| m.borrow_mut().clear());
    MENU_SUBMENU_LAYOUT.with(|m| m.borrow_mut().clear());
}

fn alloc_submenu_id() -> u64 {
    let id = NEXT_SUBMENU_ID.get();
    NEXT_SUBMENU_ID.set(id.saturating_add(1));
    id
}

pub fn register_submenu_items(submenu_id: u64, rows: &[(u16, MenuItemKind, bool)]) {
    let layout: Vec<(u16, MenuItemKind)> = rows.iter().map(|(id, kind, _)| (*id, *kind)).collect();
    MENU_SUBMENU_LAYOUT.with(|map| {
        map.borrow_mut().insert(submenu_id, layout);
    });
    for (id, kind, checked) in rows {
        MENU_ITEM_KINDS.with(|map| {
            map.borrow_mut().insert(*id, *kind);
        });
        MENU_ITEM_SUBMENU.with(|map| {
            map.borrow_mut().insert(*id, submenu_id);
        });
        MENU_ITEM_CHECKED.with(|map| {
            map.borrow_mut().insert(*id, *checked);
        });
    }
}

pub fn register_menu_item(id: u16, kind: MenuItemKind, submenu_id: u64, checked: bool) {
    MENU_ITEM_KINDS.with(|map| {
        map.borrow_mut().insert(id, kind);
    });
    MENU_ITEM_SUBMENU.with(|map| {
        map.borrow_mut().insert(id, submenu_id);
    });
    MENU_ITEM_CHECKED.with(|map| {
        map.borrow_mut().insert(id, checked);
    });
    MENU_SUBMENU_LAYOUT.with(|map| {
        map.borrow_mut()
            .entry(submenu_id)
            .or_default()
            .push((id, kind));
    });
}

pub fn begin_submenu_registration() -> u64 {
    alloc_submenu_id()
}

pub fn register_submenu_separator(submenu_id: u64) {
    MENU_SUBMENU_LAYOUT.with(|map| {
        map.borrow_mut()
            .entry(submenu_id)
            .or_default()
            .push((0, MenuItemKind::Separator));
    });
}

pub fn apply_menu_item_state(id: u16, checked: bool) {
    menu_set_item_checked(id, checked);
    menu_sync_item_checked(id, checked);
}

pub fn menu_sync_item_checked(id: u16, checked: bool) {
    MENU_ITEM_CHECKED.with(|map| {
        map.borrow_mut().insert(id, checked);
    });
}

fn sync_frame_menu_bar_memory(frame_handle: NativeHandle, id: u16, checked: bool) {
    let frame_handle = resolve_frame_handle(frame_handle);
    let Some(data_rc) = FRAME_DATA_WEAK.with(|map| {
        map.borrow()
            .get(&frame_handle)
            .and_then(|weak| weak.upgrade())
    }) else {
        return;
    };
    let mut inner = data_rc.borrow_mut();
    if let Some(ref mut bar) = inner.menu_bar {
        bar.set_item_checked_memory(id, checked);
    }
}

fn radio_group_ids(submenu_id: u64, id: u16) -> Vec<u16> {
    MENU_SUBMENU_LAYOUT.with(|map| {
        let borrowed = map.borrow();
        let Some(layout) = borrowed.get(&submenu_id) else {
            return vec![id];
        };
        let Some(start) = layout.iter().position(|(item_id, _)| *item_id == id) else {
            return vec![id];
        };
        let mut end = start + 1;
        while end < layout.len() && layout[end].1 == MenuItemKind::Radio {
            end += 1;
        }
        let mut begin = start;
        while begin > 0 && layout[begin - 1].1 == MenuItemKind::Radio {
            begin -= 1;
        }
        layout[begin..end]
            .iter()
            .filter(|(_, kind)| *kind == MenuItemKind::Radio)
            .map(|(item_id, _)| *item_id)
            .collect()
    })
}

/// Apply Win32-like check/radio semantics before dispatching the menu command.
pub fn menu_item_will_dispatch(frame_handle: NativeHandle, command_id: u16) {
    let kind = MENU_ITEM_KINDS.with(|map| map.borrow().get(&command_id).copied());
    let Some(kind) = kind else {
        return;
    };
    match kind {
        MenuItemKind::Check => {
            let current = MENU_ITEM_CHECKED
                .with(|map| map.borrow().get(&command_id).copied().unwrap_or(false));
            let new_state = !current;
            apply_menu_item_state(command_id, new_state);
            sync_frame_menu_bar_memory(frame_handle, command_id, new_state);
        }
        MenuItemKind::Radio => {
            let submenu_id = MENU_ITEM_SUBMENU.with(|map| map.borrow().get(&command_id).copied());
            let Some(submenu_id) = submenu_id else {
                apply_menu_item_state(command_id, true);
                sync_frame_menu_bar_memory(frame_handle, command_id, true);
                return;
            };
            for item_id in radio_group_ids(submenu_id, command_id) {
                let checked = item_id == command_id;
                apply_menu_item_state(item_id, checked);
                sync_frame_menu_bar_memory(frame_handle, item_id, checked);
            }
        }
        MenuItemKind::Normal | MenuItemKind::Separator => {}
    }
}
