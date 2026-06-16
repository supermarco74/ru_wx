//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Button with drop-down menu (`wxMenuButton`).

use std::cell::RefCell;
use std::rc::Rc;

use crate::controls::button::Button;
use crate::core::widget::{WidgetRef, Window};
use crate::window::frame::Frame;
use crate::window::menu::Menu;

/// Button that pops up a menu (`wxMenuButton`).
#[derive(Clone)]
pub struct MenuButton {
    button: Button,
    menu: Rc<RefCell<Menu>>,
}

impl MenuButton {
    pub fn new<W: Window>(parent: &W, label: &str) -> Self {
        Self {
            button: Button::new(parent, label),
            menu: Rc::new(RefCell::new(Menu::new(label))),
        }
    }

    pub fn menu(&self) -> std::cell::Ref<'_, Menu> {
        self.menu.borrow()
    }

    pub fn menu_mut(&self) -> std::cell::RefMut<'_, Menu> {
        self.menu.borrow_mut()
    }

    pub fn bind_popup(&self, frame: &Frame) {
        let menu = self.menu.clone();
        let fid = self.button.id();
        let f = frame.clone();
        frame.register_command_handler(fid, Box::new(move || {
            #[cfg(target_os = "windows")]
            menu.borrow().popup_at_cursor(f.hwnd());
            #[cfg(not(target_os = "windows"))]
            let _ = (&menu, &f);
        }));
    }

    pub fn as_widget_ref(&self) -> WidgetRef {
        self.button.as_widget_ref()
    }
}
