//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Simple HTML list box (`wxSimpleHtmlListBox`).
//!
//! Stores lightweight HTML snippets; renders plain text labels on Win32.

use std::cell::RefCell;
use std::rc::Rc;

use crate::controls::list_box::ListBox;
use crate::core::widget::{WidgetRef, Window};

#[derive(Clone)]
pub struct SimpleHtmlListBox {
    list: ListBox,
    html_items: Rc<RefCell<Vec<String>>>,
}

impl SimpleHtmlListBox {
    pub fn new<W: Window>(parent: &W) -> Self {
        Self {
            list: ListBox::new(parent),
            html_items: Rc::new(RefCell::new(Vec::new())),
        }
    }

    pub fn append(&self, html: &str) {
        self.html_items.borrow_mut().push(html.to_string());
        let plain = strip_html(html);
        self.list.append(&plain);
    }

    pub fn selection(&self) -> Option<usize> {
        self.list.get_selection()
    }

    pub fn html_at(&self, index: usize) -> Option<String> {
        self.html_items.borrow().get(index).cloned()
    }

    pub fn as_widget_ref(&self) -> WidgetRef {
        self.list.as_widget_ref()
    }
}

fn strip_html(html: &str) -> String {
    let mut out = String::new();
    let mut in_tag = false;
    for ch in html.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(ch),
            _ => {}
        }
    }
    out.trim().to_string()
}
