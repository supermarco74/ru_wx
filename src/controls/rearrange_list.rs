//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Reorderable list (`wxRearrangeList`).

use crate::controls::button::Button;
use crate::controls::list_box::ListBox;
use crate::core::widget::{WidgetRef, Window};
use crate::window::frame::Frame;

#[derive(Clone)]
pub struct RearrangeList {
    list: ListBox,
    up: Button,
    down: Button,
    items: std::rc::Rc<std::cell::RefCell<Vec<String>>>,
}

impl RearrangeList {
    pub fn new<W: Window>(parent: &W, frame: &Frame, items: &[&str]) -> Self {
        let list = ListBox::new(parent);
        let stored: Vec<String> = items.iter().map(|s| (*s).to_string()).collect();
        for item in &stored {
            list.append(item);
        }
        let items_rc = std::rc::Rc::new(std::cell::RefCell::new(stored));
        let up = Button::new(parent, "Up");
        let down = Button::new(parent, "Down");
        let list_c = list.clone();
        let data = items_rc.clone();
        let uid = up.id();
        let f = frame.clone();
        f.register_command_handler(uid, Box::new(move || {
            if let Some(idx) = list_c.get_selection() {
                if idx > 0 {
                    let mut v = data.borrow_mut();
                    v.swap(idx, idx - 1);
                    refresh_list(&list_c, &v);
                    list_c.set_selection(idx - 1);
                }
            }
        }));
        let list_c2 = list.clone();
        let data2 = items_rc.clone();
        let did = down.id();
        let f2 = frame.clone();
        f2.register_command_handler(did, Box::new(move || {
            if let Some(idx) = list_c2.get_selection() {
                let mut v = data2.borrow_mut();
                if idx + 1 < v.len() {
                    v.swap(idx, idx + 1);
                    refresh_list(&list_c2, &v);
                    list_c2.set_selection(idx + 1);
                }
            }
        }));
        Self {
            list,
            up,
            down,
            items: items_rc,
        }
    }

    pub fn items(&self) -> Vec<String> {
        self.items.borrow().clone()
    }

    pub fn list_widget(&self) -> WidgetRef {
        self.list.as_widget_ref()
    }

    pub fn up_widget(&self) -> WidgetRef {
        self.up.as_widget_ref()
    }

    pub fn down_widget(&self) -> WidgetRef {
        self.down.as_widget_ref()
    }
}

fn refresh_list(list: &ListBox, items: &[String]) {
    list.clear();
    for item in items {
        list.append(item);
    }
}
